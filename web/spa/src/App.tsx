import { Route, Router } from "@solidjs/router";
import { lazy } from "solid-js";
import { AppShell } from "~/components/AppShell";
import { ItemDetail } from "~/routes/ItemDetail";
import { Library } from "~/routes/Library";
import { NotFound } from "~/routes/NotFound";
import { Pair } from "~/routes/Pair";
import { Projects } from "~/routes/Projects";
import { Prompts } from "~/routes/Prompts";
import { Settings } from "~/routes/Settings";
import { Vocabulary } from "~/routes/Vocabulary";

/**
 * The editor route, and the only lazy boundary.
 *
 * TipTap plus ProseMirror is the largest thing the dashboard imports, and this was the
 * measured split point in the previous implementation. R-FE-3 makes it a rule: the editor
 * stack must not load on any other route, so a user browsing their library never pays for
 * an editor they did not open.
 */
const PromptEditor = lazy(() => import("~/routes/PromptEditor"));

/**
 * The route map, mirroring the previous implementation exactly (R-FE-2, Inventory §6).
 *
 * `/pair` is registered above the catch-all and loaded **eagerly**. It is the sanctioned
 * fallback pairing path for unpacked extension installs (D11), and the extension's content
 * script injects at document load — a lazily-loaded page would not have rendered the
 * handoff element by the time the script looks for it.
 */
export function App() {
  return (
    <Router root={AppShell}>
      <Route path="/" component={Library} />
      <Route path="/items/:id" component={ItemDetail} />
      <Route path="/projects" component={Projects} />
      <Route path="/prompts" component={Prompts} />
      <Route path="/prompts/:id" component={PromptEditor} />
      <Route path="/vocabulary" component={Vocabulary} />
      <Route path="/settings" component={Settings} />
      <Route path="/pair" component={Pair} />
      <Route path="*" component={NotFound} />
    </Router>
  );
}
