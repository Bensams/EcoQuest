import { useEffect, useRef, useState } from 'react';
import { Camera, CameraOff } from 'lucide-react';
import { Button } from '../ui/Button';
import { cameraErrorMessage, createNativeQrDetector, decodeQrFromVideo } from '../../lib/qr';

type Props = {
  onDetect: (code: string) => void;
  paused?: boolean;
};

export function LiveQrScanner({ onDetect, paused = false }: Props) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const onDetectRef = useRef(onDetect);
  const lastCodeRef = useRef<string | null>(null);
  const [active, setActive] = useState(false);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  onDetectRef.current = onDetect;

  const stop = () => {
    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;
    const video = videoRef.current;
    if (video) video.srcObject = null;
    setActive(false);
    setStarting(false);
  };

  const start = async () => {
    if (!navigator.mediaDevices?.getUserMedia) {
      setError('This browser cannot open the camera. Type the code instead.');
      return;
    }
    setError(null);
    setStarting(true);
    lastCodeRef.current = null;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: false,
        video: { facingMode: { ideal: 'environment' }, width: { ideal: 1280 }, height: { ideal: 720 } },
      }).catch(() => navigator.mediaDevices.getUserMedia({ audio: false, video: true }));
      streamRef.current = stream;
      const video = videoRef.current;
      if (!video) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }
      video.srcObject = stream;
      await video.play();
      setActive(true);
    } catch (err) {
      stop();
      setError(cameraErrorMessage(err));
    } finally {
      setStarting(false);
    }
  };

  useEffect(() => () => stop(), []);

  useEffect(() => {
    if (!active || paused) return;
    const video = videoRef.current;
    if (!video) return;
    let cancelled = false;
    let frame = 0;
    let inflight = false;
    let detector: Awaited<ReturnType<typeof createNativeQrDetector>> = null;

    const finish = (value: string | null) => {
      if (!value || value === lastCodeRef.current) return;
      lastCodeRef.current = value;
      streamRef.current?.getTracks().forEach((track) => track.stop());
      streamRef.current = null;
      if (videoRef.current) videoRef.current.srcObject = null;
      setActive(false);
      onDetectRef.current(value);
    };

    const tick = () => {
      if (cancelled) return;
      frame = requestAnimationFrame(tick);
      if (inflight || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) return;

      if (detector) {
        inflight = true;
        void detector
          .detect(video)
          .then((values) => {
            if (!cancelled) finish(values[0]?.rawValue.trim() || null);
          })
          .catch(() => {
            if (!cancelled) finish(decodeQrFromVideo(video));
          })
          .finally(() => {
            inflight = false;
          });
        return;
      }
      finish(decodeQrFromVideo(video));
    };

    void createNativeQrDetector().then((created) => {
      if (!cancelled) detector = created;
    });
    frame = requestAnimationFrame(tick);
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [active, paused]);

  return (
    <div className="grid gap-3">
      <div className="relative overflow-hidden rounded-lg bg-forest">
        <video
          ref={videoRef}
          className="aspect-square w-full object-cover"
          muted
          playsInline
          autoPlay
        />
        {!active && (
          <div className="absolute inset-0 grid place-items-center bg-forest/80 px-4 text-center text-sm text-sage-soft">
            {starting ? 'Opening camera…' : 'Start the camera to scan a check-in QR.'}
          </div>
        )}
        {active && (
          <div className="pointer-events-none absolute inset-0 grid place-items-center">
            <div className="size-2/3 rounded-xl border-2 border-white/80 shadow-[0_0_0_9999px_rgba(26,46,34,0.35)]" />
          </div>
        )}
        {active && paused && (
          <div className="absolute inset-x-0 bottom-0 bg-forest/70 px-3 py-2 text-center text-xs text-sage-soft">
            Checking in…
          </div>
        )}
      </div>
      {error && <p className="text-sm text-red-700">{error}</p>}
      <div className="flex flex-wrap gap-2">
        {active ? (
          <Button type="button" variant="secondary" onClick={stop}>
            <CameraOff className="size-4" aria-hidden="true" /> Stop camera
          </Button>
        ) : (
          <Button type="button" onClick={() => void start()} disabled={starting}>
            <Camera className="size-4" aria-hidden="true" /> {starting ? 'Starting…' : 'Start camera'}
          </Button>
        )}
      </div>
    </div>
  );
}
