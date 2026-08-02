/**
 * Copies the repo's committed docs into the site's content collection.
 *
 * The site does not own any documentation. `docs/` stays the single source of truth
 * (R-DEL-18: docs and code must not diverge silently), and a copy that a human edits
 * would be exactly the silent divergence that rule exists to prevent. So the synced
 * directories are gitignored and rebuilt from scratch on every `dev`, `build` and
 * `typecheck` — edit `docs/`, never `src/content/docs/architecture/`.
 *
 * The copy is not verbatim. Two things are rewritten:
 *
 *   1. Inter-document links. The docs link each other as `[ARCH-00](00-architecture-overview.md)`
 *      because that is what resolves on GitHub. On the site those must lose the `.md`
 *      or they 404.
 *   2. Links that escape the doc set — `../../LICENSE`, `../README.md` — have no page to
 *      point at. They become absolute GitHub URLs so they keep working.
 */

import { readFile, mkdir, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const SITE_ROOT = fileURLToPath(new URL("..", import.meta.url));
const REPO_ROOT = join(SITE_ROOT, "..", "..");
const DOCS = join(REPO_ROOT, "docs");
const OUT = join(SITE_ROOT, "src", "content", "docs");

/** The repo blob root, for links that point outside the published doc set. */
const GITHUB_BLOB = "https://github.com/rescueahero4/Curio/blob/master";

/**
 * What gets published, and where it lands.
 *
 * An allowlist, not a glob-minus-exclusions: `docs/` also holds `_plan/`, `_ref/` and
 * `_convo/`, which are gitignored planning material full of links into the old Curiol
 * tree. A pattern that swept the directory would publish them the moment someone's
 * working copy had them. Naming each file means a new doc is a deliberate act.
 */
const PAGES = [
  // Architecture — ARCH-00..08 plus the two appendices. TEMPLATE.md is excluded: its
  // frontmatter title is the literal placeholder `<Title>`.
  ["architecture/00-architecture-overview.md", "architecture/00-architecture-overview.md"],
  ["architecture/01-backend-architecture.md", "architecture/01-backend-architecture.md"],
  ["architecture/02-data-architecture.md", "architecture/02-data-architecture.md"],
  ["architecture/03-frontend-architecture.md", "architecture/03-frontend-architecture.md"],
  ["architecture/04-extension-architecture.md", "architecture/04-extension-architecture.md"],
  ["architecture/05-mcp-architecture.md", "architecture/05-mcp-architecture.md"],
  ["architecture/06-security-architecture.md", "architecture/06-security-architecture.md"],
  ["architecture/07-delivery-open-source.md", "architecture/07-delivery-open-source.md"],
  ["architecture/08-parity-matrix.md", "architecture/08-parity-matrix.md"],
  ["architecture/appendix-parity-inventory.md", "architecture/appendix-parity-inventory.md"],
  ["architecture/D0-report.md", "architecture/d0-report.md"],

  // Product.
  ["PRD-01-Foundations.md", "product/prd-01-foundations.md"],
];

/** Frontmatter keys Starlight understands. Everything else is dropped from the copy. */
const STARLIGHT_KEYS = new Set(["title", "description", "sidebar", "tableOfContents"]);

/**
 * Strips the docs' contract frontmatter down to what Starlight consumes.
 *
 * The architecture documents carry `id`, `status`, `version`, `depends_on`,
 * `source_of_truth` and friends. Starlight's schema drops unknown keys silently, but
 * `source_of_truth` names `docs/_plan/` files that are deliberately not published —
 * leaving it in the built HTML would advertise paths no reader can open.
 *
 * `id` and `status` are preserved as visible context instead, because a reader landing
 * on a draft contract document should be told it is one.
 */
function rewriteFrontmatter(raw, sourcePath) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(raw);
  if (!match) {
    throw new Error(
      `${sourcePath} has no frontmatter. Starlight requires at least a title — add one, or drop the file from PAGES.`,
    );
  }

  const body = raw.slice(match[0].length);
  const lines = match[1].split(/\r?\n/);

  const kept = [];
  let id = null;
  let status = null;
  let title = null;

  for (const line of lines) {
    // Top-level keys only. Continuation lines (list items, nested maps) belong to
    // whichever key preceded them, and every key we keep is a scalar.
    const kv = /^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$/.exec(line);
    if (!kv) continue;

    const [, key, value] = kv;
    if (key === "id") id = value.trim();
    if (key === "status") status = value.trim();
    if (key === "title") title = value.trim();
    if (STARLIGHT_KEYS.has(key) && value.trim() !== "") kept.push(`${key}: ${value}`);
  }

  if (!title) {
    throw new Error(`${sourcePath} has frontmatter but no title. Starlight cannot build a page without one.`);
  }

  // The doc id (ARCH-04) is far more useful in the sidebar and search than the prose
  // title, which is long and reads the same across the set.
  if (id) kept.push(`sidebar:\n  label: "${id} — ${title.replace(/^["']|["']$/g, "")}"`);

  const banner =
    status && status !== "accepted"
      ? `:::caution[Status: ${status}]\nThis is a contract-level document under active revision. Rule IDs are stable; wording may change.\n:::\n\n`
      : "";

  return `---\n${kept.join("\n")}\n---\n\n${banner}${body}`;
}

/** Where each published doc lands, keyed by its path relative to `docs/`. */
const DEST_BY_SOURCE = new Map(PAGES.map(([from, to]) => [from, to]));

/** POSIX separators regardless of host OS — these become URLs. */
const posix = (p) => p.split("\\").join("/");

/**
 * Rewrites markdown links so they resolve on the site instead of on GitHub.
 *
 * The links in `docs/` are written to work in the GitHub file browser, where every
 * document sits at its repo path. Three things break that on the site:
 *
 *   1. Documents move. `docs/PRD-01-Foundations.md` is published at `product/`, so its
 *      `architecture/00-...md` link is a directory level off once copied.
 *   2. Page URLs are directories (`trailingSlash: "always"`), so a sibling link that
 *      looks relative — `07-delivery-open-source` — resolves *inside* the current page,
 *      not beside it. GitHub Pages has no redirect to rescue it; it is a plain 404.
 *   3. Some targets are not published at all (TEMPLATE.md, anything above `docs/`).
 *
 * So links are re-derived from the destination layout rather than patched: resolve the
 * target to a `docs/`-relative path, look up where that document was published, and emit
 * a path from this page's *URL* to that page's URL.
 *
 * Relative URLs, not `.md` paths. Astro can resolve collection-relative markdown links
 * and apply the base prefix itself, but it does so inconsistently here — `../foo.md`
 * resolved and `./foo.md` did not, silently emitting the raw href. A relative URL needs
 * no framework cooperation and stays correct if `base` ever changes (custom domain,
 * repo rename). Page URLs are directories, so the path is computed from the directory,
 * not the page: from `/architecture/00-overview/`, a sibling is `../07-delivery/`.
 *
 * Code fences are left alone: several docs print file paths inside them, and a path in a
 * fence is sample text, not a link.
 */
function rewriteLinks(text, sourceRelPath, destRelPath) {
  const fences = [];
  // Park fenced blocks behind placeholders so the link regex cannot reach into them.
  const parked = text.replace(/```[\s\S]*?```/g, (block) => {
    fences.push(block);
    return `\u0000FENCE${fences.length - 1}\u0000`;
  });

  const sourceDir = dirname(sourceRelPath);
  /** This page's URL is a directory: `architecture/00-overview.md` → `architecture/00-overview/`. */
  const thisPageUrlDir = destRelPath.slice(0, -".md".length);

  const rewritten = parked.replace(/\]\(([^)\s]+?)(#[^)\s]*)?\)/g, (whole, target, hash = "") => {
    if (/^(https?:|mailto:|#|\/)/.test(target)) return whole;
    if (!target.endsWith(".md")) return whole;

    // Where the link points, expressed relative to `docs/`.
    const targetFromDocs = posix(relative(DOCS, join(DOCS, sourceDir, target)));

    // Above `docs/` (LICENSE, README, a crate) or inside it but unpublished
    // (TEMPLATE.md): no page exists, so point at the file on GitHub.
    if (targetFromDocs.startsWith("..")) {
      const repoRel = posix(relative(REPO_ROOT, join(DOCS, sourceDir, target)));
      return `](${GITHUB_BLOB}/${repoRel}${hash})`;
    }
    const dest = DEST_BY_SOURCE.get(targetFromDocs);
    if (!dest) return `](${GITHUB_BLOB}/docs/${targetFromDocs}${hash})`;

    // A published page: relative URL from this page's directory to that one.
    const rel = posix(relative(thisPageUrlDir, dest.slice(0, -".md".length)));

    // Several documents cite themselves — the parity matrix lists ARCH-07 in its own
    // "owning document" column. That relative path is empty, and an empty href written
    // as `${rel}/` is "/", which lands on the domain root rather than the page.
    if (rel === "") return `](${hash || "./"})`;

    return `](${rel}/${hash})`;
  });

  return rewritten.replace(/\u0000FENCE(\d+)\u0000/g, (_, i) => fences[Number(i)]);
}

async function main() {
  // Rebuilt wholesale so a doc deleted upstream cannot linger as a stale page. Only the
  // synced subdirectories are cleared — `index.mdx` beside them is hand-written and
  // committed.
  for (const dir of ["architecture", "product"]) {
    await rm(join(OUT, dir), { recursive: true, force: true });
  }

  for (const [from, to] of PAGES) {
    const raw = await readFile(join(DOCS, from), "utf8");
    const withFrontmatter = rewriteFrontmatter(raw, `docs/${from}`);
    const final = rewriteLinks(withFrontmatter, from, to);

    const dest = join(OUT, to);
    await mkdir(dirname(dest), { recursive: true });
    await writeFile(dest, final, "utf8");
  }

  console.log(`synced ${PAGES.length} docs from docs/ into src/content/docs/`);
}

await main();
