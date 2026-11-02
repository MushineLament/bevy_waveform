struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var waveform_tex: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1)
var waveform_sampler: sampler;

struct WaveformParams {
    info: vec4<f32>,
    color: vec4<f32>,
    transform: vec4<f32>,
}
@group(constants::MATERIAL_BIND_GROUP) @binding(2)
var<uniform> params: WaveformParams;

// ─── 点到线段距离（UV 空间） ───
fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let ab_x = bx - ax;
    let ab_y = by - ay;
    let ap_x = px - ax;
    let ap_y = py - ay;
    let ab_dot_ab = ab_x * ab_x + ab_y * ab_y;
    // 端点重合时（min == max 且只有一列）退化为点到点
    let t = clamp((ap_x * ab_x + ap_y * ab_y) / max(ab_dot_ab, 1e-12), 0.0, 1.0);
    let cx = ax + t * ab_x;
    let cy = ay + t * ab_y;
    let dx = px - cx;
    let dy = py - cy;
    return sqrt(dx * dx + dy * dy);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let sample_count = params.info.x;
    let line_width = params.info.y;
    let pad_px = params.info.w;
    let scale_y = params.transform.x;

    let px_y = max(fwidth(uv.y), 1e-6);
    let pad_uv = clamp(pad_px * px_y, 0.0, 0.45);

    // 安全边界，防波削
    let aa_margin = (line_width * 0.5 + 1.0) * px_y;
    let max_half_height = 0.5 - pad_uv - aa_margin;
    let raw_half_height = (0.5 - pad_uv) * scale_y;
    let half_height = min(raw_half_height, max_half_height);

    // ── 相邻两列 ──
    let idx_f = uv.x * sample_count;
    let i0 = i32(clamp(floor(idx_f), 0.0, sample_count - 1.0));
    let i1 = min(i0 + 1, i32(sample_count) - 1);
    let frac = clamp(idx_f - f32(i0), 0.0, 1.0);

    let p0 = textureLoad(waveform_tex, vec2<i32>(i0, 0), 0);
    let p1 = textureLoad(waveform_tex, vec2<i32>(i1, 0), 0);

    // R=min, G=max（振幅 [-1, 1]）
    let mn0 = p0.r * 2.0 - 1.0;
    let mx0 = p0.g * 2.0 - 1.0;
    let mn1 = p1.r * 2.0 - 1.0;
    let mx1 = p1.g * 2.0 - 1.0;

    // 列 i0 / i1 的 UV x 坐标
    let x0 = f32(i0) / sample_count;
    let x1 = f32(i1) / sample_count;

    // ── ① 垂直距离：到当前像素处 [min, max] 区间的垂直距离 ──
    let y_low = 0.5 - mix(mn0, mn1, frac) * half_height;
    let y_high = 0.5 - mix(mx0, mx1, frac) * half_height;
    let y_lo = min(y_low, y_high);
    let y_hi = max(y_low, y_high);
    let vert_dist = max(0.0, max(y_lo - uv.y, uv.y - y_hi));

    // ── ② 线段距离：到 i0/i1 两条包络线段的最小距离 ──
    let y_mn0 = 0.5 - mn0 * half_height;
    let y_mn1 = 0.5 - mn1 * half_height;
    let y_mx0 = 0.5 - mx0 * half_height;
    let y_mx1 = 0.5 - mx1 * half_height;

    let d_lower = dist_to_segment(uv.x, uv.y, x0, y_mn0, x1, y_mn1);
    let d_upper = dist_to_segment(uv.x, uv.y, x0, y_mx0, x1, y_mx1);
    let seg_dist = min(d_lower, d_upper);

    // ── 综合：取两者较小值 ──
    // - 缩小时：像素在 [min, max] 区间内 → vert_dist = 0 → 填充正确
    // - 放大时：min == max，vert_dist 是点距，seg_dist 是线距 → seg_dist 更小 → 连线连续
    let dist_uv = min(vert_dist, seg_dist);
    let dist_px = dist_uv / px_y;

    let half_width = max(line_width * 0.5, 0.5);
    let line_cover = 1.0 - smoothstep(half_width - 1.0, half_width + 1.0, dist_px);

    return vec4<f32>(params.color.rgb, params.color.a * line_cover);
}
