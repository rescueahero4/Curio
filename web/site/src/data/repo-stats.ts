/**
 * The repository's star and fork counts, shared by the build and by the browser.
 *
 * Both read the same endpoint through the same formatter on purpose: the page renders the
 * build-time number so the pills are never briefly blank and still carry a count with
 * scripting off, then the browser overwrites it with the live one. A count that is
 * formatted one way at build and another at runtime would visibly rewrite itself on load
 * even when nothing had changed.
 *
 * Neither call is authenticated. GitHub allows 60 unauthenticated requests an hour per IP,
 * which is an order of magnitude more than a landing page draws from one visitor, and a
 * token in a static page would be readable by everyone who loaded it.
 */
import { REPO } from "./downloads";

/** The REST endpoint behind {@link REPO}. `api.github.com` mirrors the `/owner/name` path. */
export const REPO_API = REPO.replace("https://github.com/", "https://api.github.com/repos/");

/** What the page shows; `null` for a count that could not be read. */
export type RepoCounts = {
	stars: number | null;
	forks: number | null;
};

/**
 * Four digits is where the pill starts to look like a spec sheet, so counts round to `1.2k`
 * above 999. `.0` is dropped — `1k`, not `1.0k`.
 */
export function formatCount(value: number): string {
	if (value < 1000) return String(value);
	return `${(value / 1000).toFixed(1).replace(/\.0$/, "")}k`;
}

/**
 * Reads both counts, or returns nulls.
 *
 * Every failure path is the same one: the pills render without a number. A landing page
 * that cannot say how many stars it has is not a broken landing page, and failing the
 * Pages build over GitHub's own API being briefly unreachable would be.
 */
export async function fetchCounts(): Promise<RepoCounts> {
	const empty: RepoCounts = { stars: null, forks: null };

	try {
		const response = await fetch(REPO_API, {
			headers: { accept: "application/vnd.github+json" },
		});
		if (!response.ok) return empty;

		const body: unknown = await response.json();
		if (typeof body !== "object" || body === null) return empty;

		const record = body as Record<string, unknown>;
		return {
			stars: typeof record.stargazers_count === "number" ? record.stargazers_count : null,
			forks: typeof record.forks_count === "number" ? record.forks_count : null,
		};
	} catch {
		return empty;
	}
}
