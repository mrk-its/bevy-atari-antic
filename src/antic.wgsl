[[block]]
struct View {
    view_proj: mat4x4<f32>;
    projection: mat4x4<f32>;
    world_position: vec3<f32>;
};
[[group(0), binding(0)]]
var<uniform> view: View;

[[block]]
struct Mesh {
    transform: mat4x4<f32>;
};
[[group(2), binding(0)]]
var<uniform> mesh: Mesh;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1),interpolate(flat)]] custom: vec4<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(1),interpolate(flat)]] custom: vec4<f32>;
    [[location(2)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.transform * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = view.view_proj * world_position;
    out.uv = vertex.uv;
    out.custom = vertex.custom;
    return out;
}

struct GTIA1 {
    color_regs: array<vec4<i32>, 2>;
    colpm: vec4<i32>;
};

struct GTIA2 {
    player_size: vec4<i32>;
    missile_size: vec4<i32>;
    grafp: vec4<i32>;
};

struct GTIA3 {
    hposp: vec4<f32>;
    hposm: vec4<f32>;
    prior: vec4<i32>;  // [prior, unused, grafm, unused]
};

[[block]]
struct GTIA1Regs {
    regs: array<GTIA1, 240>;
};

[[block]]
struct GTIA2Regs {
    regs: array<GTIA2, 240>;
};

[[block]]
struct GTIA3Regs {
    regs: array<GTIA3, 240>;
};

[[block]]
struct Palette {
    palette: array<vec4<f32>, 256>;
};
[[block]]
struct Memory {
    memory: array<vec4<u32>, 1024>;
};
[[group(1), binding(0)]]
var<uniform> gtia1_regs: GTIA1Regs;

[[group(1), binding(1)]]
var<uniform> gtia2_regs: GTIA2Regs;

[[group(1), binding(2)]]
var<uniform> gtia3_regs: GTIA3Regs;

[[group(1), binding(3)]]
var<uniform> palette: Palette;

[[group(1), binding(4)]]
var<uniform> memory1: Memory;


fn get_color_reg(scan_line: i32, k: i32) -> i32 {
    return gtia1_regs.regs[0].color_regs[k >> 2u][k & 3];
}

fn get_gtia_colpm(scan_line: i32, k: i32) -> i32 {
    return gtia1_regs.regs[scan_line].colpm[k];
}

fn get_missile_pixels(px: vec4<f32>, scan_line: i32, hpos: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(0.0);
}

fn get_player_pixels(px: vec4<f32>, scan_line: i32, hpos: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(0.0);
}

fn get_memory(offset: i32) -> i32 {
    let pixel = offset / 16;
    return i32((memory1.memory[pixel][(offset / 4) & 3] >> u32((offset & 3) * 8)) & 0xffu);
}

[[stage(fragment)]]
fn fragment(in: VertexOutput) -> [[location(0)]] vec4<f32> {

    let c0 = i32(in.custom[0]);
    let c1 = i32(in.custom[1]);

    let mode = c0 & 0xff;
    let start_scan_line = (c0 >> 8u) & 0xff;
    let line_height = (c0 >> 16u) & 0xff;

    let hscrol = c1 & 0xff;
    let line_voffset = (c1 >> 8u) & 0xff;
    let line_width = f32((c1 >> 16u) & 0xff) * 2.0;

    let video_memory_offset = i32(in.custom[2]);
    let charset_memory_offset = i32(in.custom[3]);

    let x = in.uv[0] * 384.0;
    let px = x - 192.0 + line_width / 2.0;

    let px_scrolled = px + f32(hscrol);  // pixel x position
    let cy = i32(in.uv[1] * f32(line_height) * 0.99);
    let y = cy + line_voffset;
    var hires = false;

    let scan_line = start_scan_line + cy;

    let hpos_offs = vec4<f32>(line_width / 2.0 - 256.0);
    let hposp = vec4<f32>(gtia3_regs.regs[scan_line].hposp);
    let hposm = vec4<f32>(gtia3_regs.regs[scan_line].hposm) * 2.0 + hpos_offs;

    var color_reg_index = 0; // bg_color
    let prior = gtia3_regs.regs[scan_line].prior[0];
    let gtia_mode = prior >> 6u;
    var color_reg = 0;

    if(mode == 0x0 || px < 0.0 || px >= line_width) {

    } elseif(mode == 2) {
        let w = px_scrolled / 8.0;
        let n = i32(w);
        let frac = w - f32(n);

        let c = get_memory(video_memory_offset + n);
        let inv = c >> 7u;
        let offs = (c & 0x7f) * 8 + y;
        var byte = get_memory(charset_memory_offset + offs);

        if(gtia_mode == 0) {
            let bit_offs = 7u - u32(frac * 8.0);
            let pixel_val = (((byte >> bit_offs) & 1) ^ inv);
            color_reg_index = 3 - pixel_val;  // pf2 pf1
            hires = true;
        } else {
            let bit_offs = 4u - u32(frac * 2.0) * 4u; // nibble offset
            let value = (byte >> bit_offs) & 0xf;
            if(gtia_mode == 1) {
                color_reg = value | get_color_reg(scan_line, 0) & 0xf0;
            } elseif(gtia_mode == 3) {
                color_reg = value << 4u;
                if(color_reg > 0) {
                    color_reg = color_reg | (get_color_reg(scan_line, 0) & 0xf);
                }
            } elseif(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } elseif(value < 8) {
                    color_reg = get_gtia_colpm(scan_line, value - 4);
                } else {
                    color_reg = get_color_reg(scan_line, 0);
                }
            };
        };
    }
    let pri0 = (prior & 1) > 0;
    let pri1 = (prior & 2) > 0;
    let pri2 = (prior & 4) > 0;
    let pri3 = (prior & 8) > 0;

    let pri01 = pri0 || pri1;
    let pri12 = pri1 || pri2;
    let pri23 = pri2 || pri3;
    let pri03 = pri0 || pri3;

    let vpx = vec4<f32>(px);
    let m = get_missile_pixels(vpx, scan_line, hposm);
    let m0 = m[0] > 0.0;
    let m1 = m[1] > 0.0;
    let m2 = m[2] > 0.0;
    let m3 = m[3] > 0.0;

    let p5 = (prior & 0x10) > 0;

    let p = get_player_pixels(vpx, scan_line, hposp);

    let p_ = vec4<bool>(p + f32(!p5) * m);
    let p0 = p_[0];
    let p1 = p_[1];
    let p2 = p_[2];
    let p3 = p_[3];

    let pf0 = color_reg_index == 1;
    let pf1 = !hires && color_reg_index == 2;
    let pf2 = hires || color_reg_index == 3;
    let pf3 = color_reg_index == 4 || p5 && (m0 || m1 || m2 || m3);

    let p01 = p0 || p1;
    let p23 = p2 || p3;
    let pf01 = pf0 || pf1;
    let pf23 = pf2 || pf3;

    let multi = (prior & 0x20) > 0;

    let sp0 = p0 && !(pf01 && pri23) && !(pri2 && pf23);
    let sp1 = p1  &&  !(pf01 && pri23) && !(pri2 && pf23)  &&  (!p0 || multi);
    let sp2 = p2  &&  !p01  &&  !(pf23 && pri12) && !(pf01 && !pri0);
    let sp3 = p3  &&  !p01  &&  !(pf23 && pri12) && !(pf01 && !pri0)  &&  (!p2 || multi);
    let sf3 = pf3  &&  !(p23 && pri03)  &&  !(p01 && !pri2);
    let sf0 = pf0  &&  !(p23 && pri0)  &&  !(p01 && pri01)  &&  !sf3;
    let sf1 = pf1  &&  !(p23 && pri0)  &&  !(p01 && pri01)  &&  !sf3;
    let sf2 = pf2  &&  !(p23 && pri03)  &&  !(p01 && !pri2)  &&  !sf3;
    let sb = !p01  &&  !p23  &&  !pf01  &&  !pf23;

    if(sp0) {color_reg = color_reg | get_gtia_colpm(scan_line, 0);};
    if(sp1) {color_reg = color_reg | get_gtia_colpm(scan_line, 1);};
    if(sp2) {color_reg = color_reg | get_gtia_colpm(scan_line, 2);};
    if(sp3) {color_reg = color_reg | get_gtia_colpm(scan_line, 3);};
    if(sf0) {color_reg = color_reg | get_color_reg(scan_line, 1);};
    if(sf1) {color_reg = color_reg | get_color_reg(scan_line, 2);};
    if(sf2) {color_reg = color_reg | get_color_reg(scan_line, 3);};
    if(sf3) {color_reg = color_reg | get_color_reg(scan_line, 4);};
    if(sb && gtia_mode == 0) {color_reg = color_reg | get_color_reg(scan_line, 0);};

    // color_reg = get_color_reg(scan_line, color_reg_index);

    if(hires && color_reg_index == 2) {
        color_reg = (color_reg & 0xf0) | (get_color_reg(scan_line, 2) & 0xf);
    }
    // var out_color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // if(color_reg_index == 3) {
    //     out_color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // } elseif(color_reg_index == 2) {
    //     out_color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    // }
    // return out_color;

    return palette.palette[color_reg];
}
