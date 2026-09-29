/**
 * Watermark logo shown over the playing content.
 *
 * Rendered as a non-interactive overlay anchored to the bottom-right corner
 * of the player at 20% opacity, so the page underneath remains fully visible
 * and the logo never intercepts clicks. PlayerView only mounts this while a
 * URL is playing, so it never appears on the idle screen.
 */
const PlayerLogo = () => {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute bottom-8 right-10 z-10 select-none text-right opacity-20"
    >
      {/* Logo — bottom right, thin weight for a refined look */}
      <div className="text-sm font-light uppercase tracking-[0.35em] text-white sm:text-base">
        PanonView <span className="text-white/75">player</span>
      </div>
    </div>
  );
};
export default PlayerLogo;