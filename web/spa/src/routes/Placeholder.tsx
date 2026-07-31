/**
 * A route that exists so the route map is real, and says which phase fills it in.
 *
 * The map itself is a contract (R-FE-2): the paths, their order, which one is lazy and
 * which is eager are all specified. Registering them now means the routing behavior can be
 * verified before the screens exist, rather than being retrofitted alongside them.
 */
export function Placeholder(props: { title: string; phase: string }) {
  return (
    <section class="flex flex-col gap-3">
      <h1 class="text-xl font-semibold">{props.title}</h1>
      <p style={{ color: "var(--color-muted)" }}>Lands in {props.phase}.</p>
    </section>
  );
}
