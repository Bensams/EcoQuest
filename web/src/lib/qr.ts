import jsQR from 'jsqr';

type BarcodeDetectorLike = { detect(source: ImageBitmapSource): Promise<{ rawValue: string }[]> };

type BarcodeDetectorCtor = {
  new (options?: { formats?: string[] }): BarcodeDetectorLike;
  getSupportedFormats?: () => Promise<string[]>;
};

declare global {
  interface Window {
    BarcodeDetector?: BarcodeDetectorCtor;
  }
}

export async function createNativeQrDetector(): Promise<BarcodeDetectorLike | null> {
  const Ctor = window.BarcodeDetector;
  if (!Ctor) return null;
  try {
    const formats = await Ctor.getSupportedFormats?.();
    if (formats && !formats.includes('qr_code')) return null;
    return new Ctor({ formats: ['qr_code'] });
  } catch {
    try {
      return new Ctor();
    } catch {
      return null;
    }
  }
}

export function decodeQrFromImageData(image: ImageData): string | null {
  const result = jsQR(image.data, image.width, image.height, { inversionAttempts: 'attemptBoth' });
  const value = result?.data.trim();
  return value || null;
}

export async function decodeQrFromBitmapSource(
  source: ImageBitmapSource,
  detector?: BarcodeDetectorLike | null,
): Promise<string | null> {
  if (detector) {
    const values = await detector.detect(source);
    const value = values[0]?.rawValue.trim();
    if (value) return value;
  }

  const bitmap = source instanceof ImageBitmap ? source : await createImageBitmap(source);
  try {
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) return null;
    ctx.drawImage(bitmap, 0, 0);
    return decodeQrFromImageData(ctx.getImageData(0, 0, canvas.width, canvas.height));
  } finally {
    bitmap.close();
  }
}

export function decodeQrFromVideo(video: HTMLVideoElement, maxEdge = 640): string | null {
  if (video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA || !video.videoWidth) return null;
  const scale = Math.min(1, maxEdge / Math.max(video.videoWidth, video.videoHeight));
  const width = Math.max(1, Math.round(video.videoWidth * scale));
  const height = Math.max(1, Math.round(video.videoHeight * scale));
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(video, 0, 0, width, height);
  return decodeQrFromImageData(ctx.getImageData(0, 0, width, height));
}

export function cameraErrorMessage(err: unknown): string {
  const name = err instanceof DOMException ? err.name : '';
  if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
    return 'Camera permission was denied. Allow camera access, or type the code instead.';
  }
  if (name === 'NotFoundError' || name === 'OverconstrainedError') {
    return 'No camera was found on this device.';
  }
  if (name === 'NotReadableError' || name === 'TrackStartError') {
    return 'The camera is already in use by another app.';
  }
  if (name === 'SecurityError') {
    return 'Live scanning needs HTTPS or localhost.';
  }
  return err instanceof Error ? err.message : 'Could not start the camera.';
}
