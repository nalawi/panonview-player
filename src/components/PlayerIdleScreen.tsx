import { useState, useEffect, memo } from "react";
import { Fade } from "react-slideshow-image";
import "react-slideshow-image/dist/styles.css";
import mount3Bg from "../assets/mount3.jpg";
import mount1Bg from "../assets/mount1.jpg";
import mount2Bg from "../assets/mount2.jpg";
import PanonViewLogo from "./PanonViewLogo";
import { getLocalIp } from "../services/api";

const BACKGROUND_IMAGES = [mount3Bg, mount1Bg, mount2Bg];

// Memoized so the 1-second clock re-render never touches the slideshow
const BackgroundSlideshow = memo(() => (
  <div className="bg-slideshow absolute inset-0 z-0 overflow-hidden">
    <style>{`
      .bg-slideshow .react-slideshow-container,
      .bg-slideshow .react-slideshow-fade-wrapper,
      .bg-slideshow .react-slideshow-fade-images-wrap,
      .bg-slideshow .react-slideshow-fade-images-wrap > div {
        width: 100% !important;
        height: 100vh !important;
      }
    `}</style>
    <Fade
      autoplay
      infinite
      arrows={false}
      indicators={false}
      pauseOnHover={false}
      canSwipe={false}
      duration={10000}
      transitionDuration={1000}
    >
      {BACKGROUND_IMAGES.map((src) => (
        <div
          key={src}
          style={{
            width: "100%",
            height: "100vh",
            backgroundImage: `url(${src})`,
            backgroundSize: "cover",
            backgroundPosition: "center",
            backgroundRepeat: "no-repeat",
          }}
        />
      ))}
    </Fade>
  </div>
));

const PlayerIdleScreen = () => {
  const [clock, setClock] = useState(new Date());
  const [deviceIp, setDeviceIp] = useState<string | null>(null);

  useEffect(() => {
    getLocalIp().then(setDeviceIp).catch(() => setDeviceIp(null));
  }, []);

  useEffect(() => {
    const timer = setInterval(() => setClock(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  // Preload so the first fade never shows a blank frame
  useEffect(() => {
    BACKGROUND_IMAGES.forEach((src) => {
      const img = new Image();
      img.src = src;
      img.decode?.().catch(() => {});
    });
  }, []);

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
      <BackgroundSlideshow />

      {/* <div
        className="absolute inset-0 z-[3]"
        style={{
          background:
            "linear-gradient(180deg, rgba(7, 10, 18, 0.55) 0%, rgba(7, 10, 18, 0.35) 45%, rgba(7, 10, 18, 0.8) 100%)",
        }}
        aria-hidden="true"
      /> */}


      <div className="relative z-10 flex h-full flex-col items-center justify-center px-8 text-center">
        <div className="flex items-center justify-center">
          <div className="w-[clamp(180px,22vw,380px)] drop-shadow-[0_4px_24px_rgba(0,0,0,0.45)]">
            <PanonViewLogo className="w-full" />
          </div>
        </div>
        <div className="mt-8 text-3xl font-extralight tracking-[0.08em] text-white/95">
          Ready to play
        </div>
        <div className="mt-4 max-w-xl text-base font-light leading-relaxed tracking-wide text-slate-300/90">
          No page is configured yet. Add one in the admin UI or send a URL to
          the player API — your content will appear here fullscreen.
        </div>
        <div className="mt-8 rounded-lg border border-white/15 bg-white/5 px-5 py-3 font-mono text-xs font-light text-slate-300 shadow-inner backdrop-blur-sm">
          {"POST http://" + (deviceIp ?? "<device>") + ":8787/api/v1/display"}
        </div>
      </div>
      <div className="absolute bottom-8 right-10 z-10 text-right">
        <div className="flex items-baseline justify-end gap-2">
          <span className="text-4xl font-extralight leading-none tracking-tight text-white/90 sm:text-4xl tabular-nums">
            {getTimePart("hour")}
            <span className="text-white/50">:</span>
            {getTimePart("minute")}
          </span>
          {getTimePart("dayPeriod") && (
            <span className="text-xl font-light uppercase leading-none tracking-widest text-white/50 sm:text-3xl">
              {getTimePart("dayPeriod")}
            </span>
          )}
        </div>
        <div className="mt-3 text-sm font-light uppercase tracking-[0.35em] text-white/55 sm:text-xs">
          {clock.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" })}
        </div>
      </div>
    </div>
  );
};

export default PlayerIdleScreen;