import { useState, useEffect } from "react";
import abstractBg from "../assets/abstract.jpg";

const PlayerIdleScreen = () => {
  const [clock, setClock] = useState(new Date());

  // Update the clock every second to keep time accurate
  useEffect(() => {
    const timer = setInterval(() => {
      setClock(new Date());
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // Split the time into hour/minute digits and a separate AM/PM marker so
  // the period can be rendered smaller next to the large digits.
  const timeParts = new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).formatToParts(clock);
  const getTimePart = (type: Intl.DateTimeFormatPartTypes) =>
    timeParts.find((p) => p.type === type)?.value ?? "";

  return (
    <div
      className="relative h-screen w-full overflow-hidden"
      style={{
        background:
          "radial-gradient(120% 120% at 50% 0%, #16213e 0%, #0b0f19 55%, #070a12 100%)",
      }}
    >
      {/* Background artwork */}
      <div
        className="absolute inset-0 bg-cover bg-center bg-no-repeat"
        style={{ backgroundImage: `url(${abstractBg})` }}
        aria-hidden="true"
      />

      {/* Dark scrim so text stays readable over the artwork */}
      <div
        className="absolute inset-0"
        style={{
          background:
            "linear-gradient(180deg, rgba(7, 10, 18, 0.55) 0%, rgba(7, 10, 18, 0.35) 45%, rgba(7, 10, 18, 0.8) 100%)",
        }}
        aria-hidden="true"
      />

      {/* Centered status content */}
      <div className="relative z-10 flex h-full flex-col items-center justify-center px-8 text-center">
        {/* Device Icon */}
        <div
          className="flex h-24 w-24 items-center justify-center rounded-3xl"
          style={{
            background: "linear-gradient(135deg, #3b83f6b5, #6365f1bc)",
            boxShadow: "0 20px 50px -12px rgba(59, 130, 246, 0.6)",
          }}
        >
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="#ffffff"
            strokeWidth="1.4"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <rect x="2" y="4" width="20" height="14" rx="2" />
            <path d="M8 21h8" />
            <path d="M12 18v3" />
            <path d="M10 9.5l4 2.5-4 2.5z" fill="#ffffff" stroke="none" />
          </svg>
        </div>

        {/* Status Text */}
        <div className="mt-8 text-3xl font-extralight tracking-[0.08em] text-white/95">
          Ready to play
        </div>
        <div className="mt-4 max-w-xl text-base font-light leading-relaxed tracking-wide text-slate-300/90">
          No page is configured yet. Add one in the admin UI or send a URL to
          the player API — your content will appear here fullscreen.
        </div>

        {/* API Instruction Box */}
        <div className="mt-8 rounded-lg border border-white/15 bg-white/5 px-5 py-3 font-mono text-sm font-light text-slate-300 shadow-inner backdrop-blur-sm">
          {"POST http://<device>:8787/api/v1/display"}
        </div>
      </div>

      {/* Clock — bottom right, thin weight for a refined look */}
      <div className="absolute bottom-8 right-10 z-10 text-right">
        <div className="flex items-baseline justify-end gap-2">
          <span className="text-7xl font-extralight leading-none tracking-tight text-white/90 sm:text-8xl tabular-nums">
            {getTimePart("hour")}
            <span className="text-white/50">:</span>
            {getTimePart("minute")}
          </span>
          {getTimePart("dayPeriod") && (
            <span className="text-2xl font-light uppercase leading-none tracking-widest text-white/50 sm:text-3xl">
              {getTimePart("dayPeriod")}
            </span>
          )}
        </div>
        <div className="mt-3 text-sm font-light uppercase tracking-[0.35em] text-white/55 sm:text-base">
          {clock.toLocaleDateString([], {
            weekday: "long",
            month: "long",
            day: "numeric",
          })}
        </div>
      </div>
    </div>
  );
};

export default PlayerIdleScreen;