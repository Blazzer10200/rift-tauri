// Move a node to document.body (or a target) so its position:fixed/absolute
// escapes any transformed / filtered / contained ancestor — which would
// otherwise become the containing block and dislocate viewport-anchored coords.
//
//   <div use:portal style="position: fixed; left: {x}px; top: {y}px;">…</div>

export function portal(node: HTMLElement, target: HTMLElement = document.body) {
  target.appendChild(node);
  return {
    destroy() {
      if (node.isConnected) node.remove();
    },
  };
}
