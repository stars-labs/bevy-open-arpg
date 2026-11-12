// Ground-plane adaptation of PixelFX ice/frost-nova.
// Uses PixelFX's band/crystal drawing primitives, MIT, copyright 2026 Patch and Hugo Duprez.
export function groundFrostNova(core) {
  const { Rng, fit, band, crystal, clamp, smooth, TAU } = core;
  return {
    meta: {
      name: 'Ground frost nova', size: [256, 256], origin: [128, 128],
      duration: 1.25, fps: 30, loop: false, bg: 'none',
      options: { palette: [], seed: 9 },
    },
    create(options) {
      const colors = fit(options.palette, 8);
      const random = new Rng(options.seed);
      const reach = 94;
      const shards = Array.from({ length: 30 }, (_, index) => ({
        angle: index / 30 * TAU + random.range(-0.09, 0.09),
        distance: reach * random.range(0.35, 0.78),
        height: random.range(10, 21), width: random.range(3, 6),
        lean: random.range(-3, 3),
      }));
      let time = 0;
      return {
        update(delta) { time += delta; },
        draw(painter) {
          const progress = clamp(time / 1.25);
          const front = reach * (1 - Math.pow(1 - clamp(time / 0.30), 3));
          const opacity = 1 - smooth(0.55, 1.0, progress);
          band(painter, 0, 0, Math.max(1, front), 1, 3.5, opacity, colors);
          band(painter, 0, 0, Math.max(1, front * 0.72), 1, 1, opacity * 0.35, colors);
          const originX = painter.tx;
          const originY = painter.ty;
          for (const shard of shards) {
            const grow = clamp((front - shard.distance) / 18);
            if (grow <= 0 || opacity <= 0) continue;
            const x = Math.cos(shard.angle) * shard.distance;
            const y = Math.sin(shard.angle) * shard.distance;
            painter.place(originX + x, originY + y, false, shard.angle + Math.PI / 2);
            crystal(painter, 0, 0, shard.height * grow * opacity,
              shard.width * opacity, shard.lean, colors);
          }
          painter.place(originX, originY);
          for (let index = 0; index < 18; index++) {
            const angle = index / 18 * TAU + 0.3;
            const radius = front * (0.5 + (index % 4) * 0.12);
            painter.dot(Math.cos(angle) * radius, Math.sin(angle) * radius,
              colors[5], opacity * 0.55);
          }
          painter.glow(0, 0, 18, colors[4], opacity * 0.10);
        },
      };
    },
  };
}
