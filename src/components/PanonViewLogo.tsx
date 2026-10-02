/**
 * PanonView brand logo.
 *
 * The SVG keeps a fixed 196x52 viewBox, so it scales proportionally to
 * whatever width the parent (or the `className` prop) gives it — e.g.
 * `<PanonViewLogo className="w-64" />`. Used as the centered brand mark on
 * the idle screen and suitable as a non-interactive watermark overlay.
 */
const PanonViewLogo = ({ className = "" }: { className?: string }) => {
  return (
    <div
      aria-hidden="true"
      className={`pointer-events-none z-10 select-none text-right ${className}`}
    >
      <svg
        viewBox="0 0 196 52"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="h-auto w-full"
      >
        <defs>
          {/* Gradient: dark blue -> light blue */}
          <linearGradient id="panoGradient" x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor="#0877cbff" />
            <stop offset="100%" stopColor="#8eb9ffff" />
          </linearGradient>
        </defs>
        {/* Scan-alt icon vertically centered with logo text */}
        <svg x="6" y="10" width="32" height="32" viewBox="0 0 24 24" id="scan-alt" data-name="Flat Color" fill="#2ba3ffff" xmlns="http://www.w3.org/2000/svg"><g id="SVGRepo_bgCarrier" strokeWidth="0"></g><g id="SVGRepo_tracerCarrier" strokeLinecap="round" strokeLinejoin="round"></g><g id="SVGRepo_iconCarrier"><path id="secondary" d="M12,17a1,1,0,0,1-1-1V8a1,1,0,0,1,2,0v8A1,1,0,0,1,12,17Zm4-2a1,1,0,0,1-1-1V10a1,1,0,0,1,2,0v4A1,1,0,0,1,16,15ZM8,15a1,1,0,0,1-1-1V10a1,1,0,0,1,2,0v4A1,1,0,0,1,8,15Z"></path><path id="primary" d="M20,22H16a1,1,0,0,1,0-2h4V16a1,1,0,0,1,2,0v4A2,2,0,0,1,20,22ZM9,21a1,1,0,0,0-1-1H4V16a1,1,0,0,0-2,0v4a2,2,0,0,0,2,2H8A1,1,0,0,0,9,21ZM4,8V4H8A1,1,0,0,0,8,2H4A2,2,0,0,0,2,4V8A1,1,0,0,0,4,8ZM22,8V4a2,2,0,0,0-2-2H16a1,1,0,0,0,0,2h4V8a1,1,0,0,0,2,0Z"></path></g></svg>


        {/* "Pano" with gradient fill */}
        <text
          x="52"
          y="35"
          fontFamily="Inter, -apple-system, sans-serif"
          fontSize="20"
          fontWeight="600"
          fill="url(#panoGradient)"
          letterSpacing="-0.5"
        >
          Panon
        </text>
        {/* "View" follows theme text color */}
        <text
          x="110"
          y="35"
          fontFamily="Inter, -apple-system, sans-serif"
          fontSize="20"
          fontWeight="400"
          style={{ fill: 'var(--pv-text-primary, #e5e7eb)' }}
          letterSpacing="-0.5"
        >
          View
        </text>
      </svg>
    </div>
  );
};
export default PanonViewLogo;