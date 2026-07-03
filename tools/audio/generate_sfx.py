import math
import random
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "assets" / "audio"
OUT.mkdir(parents=True, exist_ok=True)

RATE = 44_100


def envelope(index, total, attack=0.012, release=0.12):
    t = index / RATE
    duration = total / RATE
    if t < attack:
        return t / attack
    tail = max(duration - t, 0.0)
    return min(1.0, tail / release)


def write_wav(name, samples):
    path = OUT / f"{name}.wav"
    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(RATE)
        frames = bytearray()
        for sample in samples:
            value = int(max(-1.0, min(1.0, sample)) * 32767)
            frames += value.to_bytes(2, "little", signed=True)
        wav.writeframes(frames)


def tone(duration, start_hz, end_hz, volume=0.4, noise=0.0, seed=0):
    rng = random.Random(seed)
    total = int(duration * RATE)
    phase = 0.0
    samples = []
    for i in range(total):
        mix = i / max(total - 1, 1)
        hz = start_hz + (end_hz - start_hz) * mix
        phase += math.tau * hz / RATE
        body = math.sin(phase) + math.sin(phase * 2.01) * 0.28
        body += (rng.random() * 2.0 - 1.0) * noise
        samples.append(body * volume * envelope(i, total))
    return samples


def chord(duration, freqs, volume=0.34, seed=0):
    rng = random.Random(seed)
    total = int(duration * RATE)
    samples = []
    for i in range(total):
        sample = sum(math.sin(math.tau * f * i / RATE) for f in freqs) / len(freqs)
        sample += (rng.random() * 2.0 - 1.0) * 0.025
        samples.append(sample * volume * envelope(i, total, attack=0.02, release=0.22))
    return samples


def mix(*tracks):
    total = max(len(track) for track in tracks)
    samples = []
    for i in range(total):
        sample = sum(track[i] if i < len(track) else 0.0 for track in tracks)
        samples.append(sample / max(len(tracks), 1))
    return samples


write_wav("hit", tone(0.16, 220, 105, 0.42, noise=0.24, seed=11))
write_wav("critical", tone(0.22, 760, 185, 0.46, noise=0.18, seed=17))
write_wav("loot", chord(0.34, [523.25, 659.25, 783.99], 0.32, seed=23))
write_wav("danger", tone(0.28, 96, 148, 0.36, noise=0.12, seed=31))
write_wav("death", tone(0.30, 180, 54, 0.38, noise=0.20, seed=43))
write_wav("skill", tone(0.22, 420, 690, 0.34, noise=0.04, seed=47))
write_wav("potion", chord(0.38, [329.63, 493.88, 659.25], 0.30, seed=53))
write_wav("utility", tone(0.18, 360, 260, 0.28, noise=0.06, seed=57))
write_wav("combo", mix(
    tone(0.30, 300, 920, 0.48, noise=0.08, seed=67),
    chord(0.30, [392.0, 587.33, 880.0], 0.28, seed=71),
))
write_wav("boss", mix(
    tone(0.58, 74, 132, 0.48, noise=0.16, seed=73),
    chord(0.58, [110.0, 146.83, 220.0], 0.24, seed=79),
))
write_wav("quest", chord(0.48, [261.63, 392.0, 523.25, 659.25], 0.34, seed=83))
write_wav("victory", chord(0.72, [392.0, 493.88, 587.33, 783.99], 0.34, seed=59))
write_wav("defeat", tone(0.78, 147, 62, 0.34, noise=0.07, seed=61))
