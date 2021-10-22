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
    [[location(1)]] uv: vec2<f32>;
    [[location(2), interpolate(flat)]] custom: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(1)]] uv: vec2<f32>;
    [[location(2), interpolate(flat)]] custom: vec4<f32>;
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
    color_regs1: vec4<i32>;
    color_regs2: vec4<i32>;
    color_pm: vec4<i32>;
};

struct GTIA2 {
    player_size: vec4<f32>;
    missile_size: vec4<f32>;
    grafp: vec4<i32>;
};

struct GTIA3 {
    hposp: vec4<f32>;
    hposm: vec4<f32>;
    prior: vec4<i32>;  // [prior, unused, grafm, unused]
};


struct MemBlock {
    data: array<vec4<u32>, 4>;
};

let memory_uniform_size: i32 = 16384;
let memory_uniform_size_blocks: i32 = 128;

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

[[group(1), binding(0)]]
var<uniform> gtia1_regs: GTIA1Regs;

[[group(1), binding(1)]]
var<uniform> gtia2_regs: GTIA2Regs;

[[group(1), binding(2)]]
var<uniform> gtia3_regs: GTIA3Regs;

[[group(1), binding(3)]]
var<uniform> palette: Palette;

[[group(1), binding(4)]]
var memory: texture_2d<u32>;

fn get_color_reg(scan_line: i32, k: i32) -> i32 {
    if(k <= 3) {
        return gtia1_regs.regs[scan_line].color_regs1[k & 3];
    } else {
        return gtia1_regs.regs[scan_line].color_regs2[k & 3];
    }
}

fn get_gtia_colpm(scan_line: i32, k: i32) -> i32 {
    return gtia1_regs.regs[scan_line].color_pm[k];
}

fn get_pm_pixels(px: vec4<f32>, w: f32, scan_line: i32, msize: vec4<f32>, hpos: vec4<f32>, data: vec4<i32>) -> vec4<f32> {
    let cond = vec4<f32>(px >= hpos) * vec4<f32>(px < hpos + msize);
    let bit = vec4<u32>(mix(vec4<f32>(w - 0.001), vec4<f32>(0.0), (px - hpos) / msize));
    return mix(vec4<f32>(0.0), vec4<f32>(((data >> bit) & vec4<i32>(1)) > vec4<i32>(0)), cond);
}

fn get_memory(offset: i32) -> i32 {
    let w = offset & 0xff;
    let h = offset >> 8u;
    let v: vec4<u32> = textureLoad(memory, vec2<i32>(w, h), 0);
    return i32(v.x & 0xffu);
 }

[[stage(fragment)]]
fn fragment(
    [[location(1)]] uv: vec2<f32>,
    [[location(2), interpolate(flat)]] custom: vec4<f32>
) -> [[location(0)]] vec4<f32> {
    let c0 = u32(custom[0]);
    let c1 = u32(custom[1]);
    let video_memory_offset = i32(custom[2]);
    let charset_memory_offset = i32(custom[3]);

    let mode = i32(c0 & 0xffu);
    let start_scan_line = i32((c0 >> 8u) & 0xffu);
    let line_height = i32((c0 >> 16u) & 0xffu);

    let hscrol = i32(c1 & 0xffu);
    let line_voffset = i32((c1 >> 8u) & 0xffu);
    let line_width = f32((c1 >> 16u) & 0xffu) * 2.0;

    let x = uv[0] * 384.0;
    let px = x - 192.0 + line_width / 2.0;

    let px_scrolled = px + f32(hscrol);  // pixel x position
    let cy = i32(uv[1] * f32(line_height) * 0.99);
    let y = cy + line_voffset;
    var hires = false;

    let scan_line = start_scan_line + cy;

    let hpos_offs = vec4<f32>(line_width / 2.0 - 256.0);
    let hposp = vec4<f32>(gtia3_regs.regs[scan_line].hposp) * 2.0 + hpos_offs;
    let hposm = vec4<f32>(gtia3_regs.regs[scan_line].hposm) * 2.0 + hpos_offs;

    var color_reg_index = 0; // bg_color
    let prior = gtia3_regs.regs[scan_line].prior[0];
    let gtia_mode = prior >> 6u;
    var color_reg = 0;

    // var ccc: vec4<f32>;

    // let bit = 31u - u32(uv[0] * 32.0);
    // let grid = u32(uv[0] * 256.0);
    // if((grid & 0x7u) == 0u) {
    //     ccc = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // } elseif(((video_memory_offset >> bit) & 1) > 0) {
    //     ccc = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // } else {
    //     ccc = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    // }

    if(mode == 0x0 || px < 0.0 || px >= line_width) {

    } elseif(mode == 2) {
        let w = px_scrolled / 8.0;
        let n = i32(w);
        let frac = w - f32(n);

        let c = get_memory(video_memory_offset + n);
        let inv = c >> 7u;
        let offs = (c & 0x7f) * 8 + y;
        let offset = charset_memory_offset + offs;
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
                color_reg = value | gtia1_regs.regs[scan_line].color_regs1[0] & 0xf0;
            } elseif(gtia_mode == 3) {
                color_reg = value << 4u;
                if(color_reg > 0) {
                    color_reg = color_reg | (gtia1_regs.regs[scan_line].color_regs1[0] & 0xf);
                }
            } elseif(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } elseif(value < 8) {
                    let idx = value - 4;
                    color_reg = gtia1_regs.regs[scan_line].color_pm[idx];
                } else {
                    color_reg = gtia1_regs.regs[scan_line].color_regs1[0];
                }
            };
        };
    } elseif(mode == 4 || mode == 5) {
        let w = px_scrolled / 8.0;
        let n = i32(w);
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u;

        let c = get_memory(video_memory_offset + n);
        let inv = c >> 7u;
        let offs = (c & 0x7f) * 8 + y;
        let byte = get_memory(charset_memory_offset + offs);
        color_reg_index = (byte >> bit_offs) & 3;
        if(inv != 0 && color_reg_index == 3) {
            color_reg_index = 4;
        };
    } elseif(mode == 6 || mode == 7) {
        let w = px_scrolled / 16.0;
        let n = i32(w);
        let frac = w - f32(n);
        let bit_offs = 7u - u32(frac * 8.0);
        var yy = y;
        if(mode == 7) {yy = yy / 2;};

        let c = get_memory(video_memory_offset + n);
        let cc = c >> 6u;
        let offs = (c & 0x3f) * 8 + yy;
        let byte = get_memory(charset_memory_offset + offs);

        if(((byte >> bit_offs) & 1) > 0) {
            color_reg_index = cc + 1;
        } else {
            color_reg_index = 0;
        };
    } elseif(mode == 10) {
        let w = px_scrolled / 16.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u; // bit offset in byte

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 3;
    } elseif(mode == 11 || mode == 12) {
        let w = px_scrolled / 16.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 7u - u32(frac * 8.0);

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 1;
    } elseif(mode == 13 || mode == 14) {
        let w = px_scrolled / 8.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u; // bit offset in byte

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 3;

    } elseif(mode == 15) {
        let w = px_scrolled / 8.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let byte = get_memory(video_memory_offset + n);

        if(gtia_mode == 0) {
            let bit_offs = 7u - u32(frac * 8.0);
            let pixel_val = (byte >> bit_offs) & 1;
            color_reg_index = 3 - pixel_val;
            hires = true;
        } else {
            let bit_offs = 4u - u32(frac * 2.0) * 4u; // nibble offset
            let value = (byte >> bit_offs) & 0xf;
            if(gtia_mode == 1) {
                color_reg = value | gtia1_regs.regs[scan_line].color_regs1[0] & 0xf0;
            } elseif(gtia_mode == 3) {
                color_reg = value << 4u;
                if(color_reg > 0) {
                    color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs1[0] & 0xf;
                };
            } elseif(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } elseif(value < 8) {
                    color_reg = gtia1_regs.regs[scan_line].color_pm[value - 4];
                } else {
                    color_reg = gtia1_regs.regs[scan_line].color_regs1[0];
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

    let missile_shift = vec4<u32>(0u, 2u, 4u, 6u);
    let mdata = vec4<i32>(gtia3_regs.regs[scan_line].prior[2]) >> missile_shift;
    let msize = gtia2_regs.regs[scan_line].missile_size;
    let m = get_pm_pixels(vpx, 2.0, scan_line, msize, hposm, mdata);

    let m0 = m[0] > 0.0;
    let m1 = m[1] > 0.0;
    let m2 = m[2] > 0.0;
    let m3 = m[3] > 0.0;

    let p5 = (prior & 0x10) > 0;

    let psize = gtia2_regs.regs[scan_line].player_size;
    let data= gtia2_regs.regs[scan_line].grafp;

    let p = get_pm_pixels(vpx, 8.0, scan_line, psize, hposp, data);

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

    if(sp0) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_pm[0];};
    if(sp1) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_pm[1];};
    if(sp2) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_pm[2];};
    if(sp3) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_pm[3];};
    if(sf0) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs1[1];};
    if(sf1) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs1[2];};
    if(sf2) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs1[3];};
    if(sf3) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs2[4 & 3];};
    if(sb && gtia_mode == 0) {color_reg = color_reg | gtia1_regs.regs[scan_line].color_regs1[0];};

    if(hires && color_reg_index == 2) {
        color_reg = (color_reg & 0xf0) | (gtia1_regs.regs[scan_line].color_regs1[2] & 0xf);
    }
    return palette.palette[color_reg];
}
