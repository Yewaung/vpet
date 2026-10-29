// The four-legged rig shared by the fox, cat, dog and rabbit: tail, far legs,
// body, near legs, then the head, drawn back to front. A form is a spec:
//   body: [cx, cy, rx, ry] or a list of them   legs: { hind, front, far, hip, r }
//   fur, paw: ramps   tail(c, dy, sway), chest(c, bodyMask, dy), head(c, dy, pose)
// The legs hang from hips that move with the body, so a bob or a breath
// bends them, and each foot is placed by the pose (see walkPose in kit.px.js).

function quad(c, S, p) {
  const by = p.bob + p.breathe;
  const L = S.legs;
  const [hf, hn, ff, fn] = p.feet;
  const foot = (x, f) => [x + f[0], GROUND + f[1]];
  S.tail(c, by, p.tail);
  leg(c, L.hind + L.far, L.hip + by, ...foot(L.hind + L.far, hf), L.r, darker(S.fur), darker(S.paw));
  leg(c, L.front + L.far, L.hip + by, ...foot(L.front + L.far, ff), L.r, darker(S.fur), darker(S.paw));
  // The body is one ellipse or several (a rump and a deeper chest).
  const body = any(...(Array.isArray(S.body[0]) ? S.body : [S.body]).map(([cx, cy, rx, ry]) => ell(cx, cy + by, rx, ry)));
  part(c, body, S.fur);
  if (S.chest) S.chest(c, body, by);
  leg(c, L.hind, L.hip + by, ...foot(L.hind, hn), L.r, S.fur, S.paw);
  leg(c, L.front, L.hip + by, ...foot(L.front, fn), L.r, S.fur, S.paw);
  S.head(c, by, p);
}

/** A pointed ear from base (x0..x1 at y) to its tip, with an inner colour and an optional dark tip. */
function ear(c, x0, x1, y, tx, ty, fur, inner, tip) {
  const m = poly([x0, y, tx, ty, x1, y]);
  part(c, m, fur, { band: 1 });
  if (inner) patch(c, poly([x0 + 2, y, tx + (x0 < tx ? 0.5 : -0.5), ty + 4, x1 - 2, y]), inner, m, { flat: inner[2] });
  if (tip) patch(c, (x, y2) => y2 < ty + 4, tip, m);
}
