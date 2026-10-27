import { useState, useEffect } from "react";
import mount3Bg from "../assets/mount3.jpg";
import mount1Bg from "../assets/mount1.jpg";
import mount2Bg from "../assets/mount2.jpg";
import PanonViewLogo from "./PanonViewLogo";
import { getLocalIp } from "../services/api";

const BACKGROUND_IMAGES = [mount3Bg, mount1Bg, mount2Bg];

const PlayerIdleScreen = () => {
  const [clock, setClock] = useState(new Date());
  const [deviceIp, setDeviceIp] = useState<string | null>(null);
  const [bgIndex, setBgIndex] = useState(0);

  useEffect(() => {
    getLocalIp().then(setDeviceIp).catch(() => setDeviceIp(null));
  }, []);

  useEffect(() => {
    const timer = setInterval(() => setClock(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    const slideshow = setInterval(() => {
      setBgIndex((i) => (i + 1) % BACKGROUND_IMAGES.length);
    }, 10000);
    return () => clearInterval(slideshow);
  }, []);

  const timeParts = new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).formatToParts(clock);
  const getTimePart = (type: Intl.DateTimeFormatPartTypes) =>
    timeParts.find((p) => p.type === type)?.value ?? "";

  return (
    <div className="relative h-screen w-full overflow-hidden" style={{
      background: "radial-gradient(120% 120% at 50% 0%, #16213e 0%, #0b0f19 55%, #070a12 100%)",
    }}>
      <div className="absolute inset-0 bg-cover bg-center bg-no-repeat transition-opacity duration-1000" style={{
        backgroundImage: `url(${BACKGROUND_IMAGES[bgIndex]})`,
      }} aria-hidden="true" />
      <div className="absolute inset-0" style={{
        background: "linear-gradient(180deg, rgba(7, 10, 18, 0.55) 0%, rgba(7, 10, 18, 0.35) 45%, rgba(7, 10, 18, 0.8) 100%)",
      }} aria-hidden="true" />
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
        <div className="mt-8 rounded-lg border border-white/15 bg-white/5 px-5 py-3 font-mono text-xm font-light text-slate-300 shadow-inner backdrop-blur-sm">
          {"POST http://" + (deviceIp ?? "<device>") + ":8787/api/v1/display"}
        </div>
      </div>
      <div className="absolute bottom-8 right-10 z-10 text-right">
        <div className="flex items-baseline justify-end gap-2">
          <span className="text-3xl font-extralight leading-none tracking-tight text-white/90 sm:text-8xl tabular-nums">
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
        <div className="mt-3 text-xm font-light uppercase tracking-[0.35em] text-white/55 sm:text-base">
          {clock.toLocaleDateString([], { weekday: "long", month: "long", day: "numeric" })}
        </div>
      </div>
    </div>
  );
};

export default PlayerIdleScreen;