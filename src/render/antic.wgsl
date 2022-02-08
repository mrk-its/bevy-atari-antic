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

struct FragmentOutput {
     [[location(0)]] color: vec4<f32>;
     [[location(1)]] collisions: vec4<u32>;
};

let memory_offset: i32 = 7680; // memory reserved for gtia regs: 240 * 32;

let COLPM0: i32 = 0x12;
let COLPF0: i32 = 0x16;
let COLBK: i32 = 0x1A;

struct Palette {
    palette: array<vec4<f32>, 256>;
};

struct AnticConfig {
    debug_scan_line: i32;
};


[[group(0), binding(0)]]
var memory: texture_2d<u32>;

[[group(0), binding(1)]]
var<uniform> palette: Palette;

[[group(0), binding(2)]]
var<uniform> antic_config: AnticConfig;

fn get_gtia_reg(scan_line: i32, k: i32) -> i32 {
    let offset = scan_line * 32 + k;
    let w = offset & 0xff;
    let h = offset >> 8u;
    let v: vec4<u32> = textureLoad(memory, vec2<i32>(w, h), 0);
    return i32(v.x & 0xffu);
}

fn get_gtia_reg4(scan_line: i32, k: i32) -> vec4<u32> {
    let offset = scan_line * 32 + k;
    let w = offset & 0xff;
    let h = offset >> 8u;
    let v1: vec4<u32> = textureLoad(memory, vec2<i32>(w, h), 0);
    let v2: vec4<u32> = textureLoad(memory, vec2<i32>(w+1, h), 0);
    let v3: vec4<u32> = textureLoad(memory, vec2<i32>(w+2, h), 0);
    let v4: vec4<u32> = textureLoad(memory, vec2<i32>(w+3, h), 0);
    return vec4<u32>(v1.x, v2.x, v3.x, v4.x);
}

fn get_pm_pixels(px: vec4<f32>, w: f32, scan_line: i32, msize: vec4<f32>, hpos: vec4<f32>, data: vec4<u32>) -> vec4<f32> {
    let cond = vec4<f32>(px >= hpos) * vec4<f32>(px < hpos + msize);
    let bit = vec4<u32>(mix(vec4<f32>(w - 0.001), vec4<f32>(0.0), (px - hpos) / msize));
    return mix(vec4<f32>(0.0), vec4<f32>(((data >> bit) & vec4<u32>(1u)) > vec4<u32>(0u)), cond);
}

fn get_memory(offset: i32) -> i32 {
    let w = (offset + memory_offset) & 0xff;
    let h = (offset + memory_offset) >> 8u;
    let v: vec4<u32> = textureLoad(memory, vec2<i32>(w, h), 0);
    return i32(v.x & 0xffu);
}

fn cond_i32(pred: bool, a: i32, b: i32) -> i32 {
    if(pred) {
        return a;
    };
    return b;
}

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = vec4<f32>(vertex.position, 1.0);
    var out: VertexOutput;
    let view_proj = mat4x4<f32>(
        vec4<f32>(2.0 / 384.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 2.0 / 240.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.001, 0.0),
        vec4<f32>(-1.0, -1.0, 1.0, 1.0)
    );
    out.clip_position = view_proj * world_position;
    out.uv = vertex.uv;
    out.custom = vertex.custom;
    return out;
}

[[stage(vertex)]]
fn collision_agg_vertex(vertex: Vertex) -> VertexOutput {
    let world_position = vec4<f32>(vertex.position, 1.0);
    var out: VertexOutput;
    let view_proj = mat4x4<f32>(
        vec4<f32>(2.0 / 128.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 2.0 / 384.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.001, 0.0),
        vec4<f32>(-1.0, -1.0, 1.0, 1.0)
    );
    out.clip_position = view_proj * world_position;
    out.uv = vertex.uv;
    out.custom = vertex.custom;
    return out;
}

[[stage(fragment)]]
fn collisions_agg_fragment(
    [[location(1)]] uv: vec2<f32>,
    [[location(2), interpolate(flat)]] custom: vec4<f32>
) -> [[location(0)]] vec4<u32> {
    var TEXTURE_HEIGHT = 32;
# ifdef T_1
    TEXTURE_HEIGHT = 1;
# endif

# ifdef T_2
    TEXTURE_HEIGHT = 2;
# endif

# ifdef T_3
    TEXTURE_HEIGHT = 3;
# endif

# ifdef T_4
    TEXTURE_HEIGHT = 4;
# endif
# ifdef T_6
    TEXTURE_HEIGHT = 6;
# endif
# ifdef T_8
    TEXTURE_HEIGHT = 8;
# endif
# ifdef T_12
    TEXTURE_HEIGHT = 12;
# endif
# ifdef T_16
    TEXTURE_HEIGHT = 16;
# endif
# ifdef T_24
    TEXTURE_HEIGHT = 24;
# endif
# ifdef T_32
    TEXTURE_HEIGHT = 32;
# endif
# ifdef T_384
    TEXTURE_HEIGHT = 384;
# endif

    let STRIP_WIDTH = 384 / TEXTURE_HEIGHT;
    let px = i32(uv.y * f32(TEXTURE_HEIGHT)) * STRIP_WIDTH;
    let py = i32(uv.x * 120.0) * 2;

    var v = vec4<u32>(0u, 0u, 0u, 0u);
    for(var x = 0; x < STRIP_WIDTH; x = x + 1) {
        let t1 = textureLoad(memory, vec2<i32>(px + x, py), 0);
        let a = t1[0] | (t1[1] << 16u);
        let b = t1[2] | (t1[3] << 16u);
        let t2 = textureLoad(memory, vec2<i32>(px + x + 1, py), 0);
        let c = t2[0] | (t2[1] << 16u);
        let d = t2[2] | (t2[3] << 16u);
        v = v | vec4<u32>(a, b, c, d);
    }
    return v;
}


[[stage(fragment)]]
fn fragment(
    [[location(1)]] uv: vec2<f32>,
    [[location(2), interpolate(flat)]] custom: vec4<f32>
) -> FragmentOutput {
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
    let hposp = vec4<f32>(get_gtia_reg4(scan_line, 0x00)) * 2.0 + hpos_offs;
    let hposm = vec4<f32>(get_gtia_reg4(scan_line, 0x04)) * 2.0 + hpos_offs;

    var color_reg_index = 0; // bg_color
    let prior = get_gtia_reg(scan_line, 0x1b);
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

    let colbk = get_gtia_reg(scan_line, COLBK);

    if(mode == 0x0 || px < 0.0 || px >= line_width) {

    } else if(mode == 2) {
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
                color_reg = value | colbk & 0xf0;
            } else if(gtia_mode == 3) {
                color_reg = value << 4u;
                if(color_reg > 0) {
                    color_reg = color_reg | (colbk & 0xf);
                }
            } else if(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } else if(value < 8) {
                    let idx = value - 4;
                    color_reg = get_gtia_reg(scan_line, COLPM0 + idx);
                } else {
                    color_reg = colbk;
                }
            };
        };
    } else if(mode == 4 || mode == 5) {
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
    } else if(mode == 6 || mode == 7) {
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
    } else if(mode == 8) {
        let w = px_scrolled / 32.0;;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u; // bit offset in byte
        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 9) {
        let w = px_scrolled / 32.0;;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 7u - u32(frac * 8.0);
        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 1;
    } else if(mode == 10) {
        let w = px_scrolled / 16.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u; // bit offset in byte

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 3;
    } else if(mode == 11 || mode == 12) {
        let w = px_scrolled / 16.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 7u - u32(frac * 8.0);

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 1;
    } else if(mode == 13 || mode == 14) {
        let w = px_scrolled / 8.0;
        let n = i32(w); // byte offset
        let frac = w - f32(n);
        let bit_offs = 6u - u32(frac * 4.0) * 2u; // bit offset in byte

        let byte = get_memory(video_memory_offset + n);
        color_reg_index = (byte >> bit_offs) & 3;

    } else if(mode == 15) {
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
                color_reg = value | colbk & 0xf0;
            } else if(gtia_mode == 3) {
                color_reg = value << 4u;
                if(color_reg > 0) {
                    color_reg = color_reg | colbk & 0xf;
                };
            } else if(gtia_mode == 2) {
                if(value < 4) {
                    color_reg_index = value + 1;
                } else if(value < 8) {
                    color_reg = get_gtia_reg(scan_line, COLPM0 + value - 4);
                } else {
                    color_reg = colbk;
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
    let mdata = vec4<u32>(u32(get_gtia_reg(scan_line, 0x11))) >> missile_shift;

    let msize_ = (vec4<u32>(u32(get_gtia_reg(scan_line, 0x0c))) >> missile_shift) & vec4<u32>(0x3u);
    let msize = vec4<f32>(vec4<i32>(4) << msize_);

    let m = get_pm_pixels(vpx, 2.0, scan_line, msize, hposm, mdata);

    let m0 = m[0] > 0.0;
    let m1 = m[1] > 0.0;
    let m2 = m[2] > 0.0;
    let m3 = m[3] > 0.0;

    let p5 = (prior & 0x10) > 0;

    let psize_ = get_gtia_reg4(scan_line, 0x08) & vec4<u32>(0x3u);
    let psize = vec4<f32>(vec4<i32>(16) << psize_);
    let data = get_gtia_reg4(scan_line, 0x0d);

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

    if(sp0) {color_reg = color_reg | get_gtia_reg(scan_line, COLPM0 + 0);};
    if(sp1) {color_reg = color_reg | get_gtia_reg(scan_line, COLPM0 + 1);};
    if(sp2) {color_reg = color_reg | get_gtia_reg(scan_line, COLPM0 + 2);};
    if(sp3) {color_reg = color_reg | get_gtia_reg(scan_line, COLPM0 + 3);};
    if(sf0) {color_reg = color_reg | get_gtia_reg(scan_line, COLPF0 + 0);};
    if(sf1) {color_reg = color_reg | get_gtia_reg(scan_line, COLPF0 + 1);};
    if(sf2) {color_reg = color_reg | get_gtia_reg(scan_line, COLPF0 + 2);};
    if(sf3) {color_reg = color_reg | get_gtia_reg(scan_line, COLPF0 + 3);};
    if(sb && gtia_mode == 0) {color_reg = color_reg | colbk;};

    if(hires && color_reg_index == 2) {
        color_reg = (color_reg & 0xf0) | (get_gtia_reg(scan_line, COLPF0 + 1) & 0xf);
    }

    // TODO - do not check collisions on HBLANK

    let p0_ = bool(p[0]);
    let p1_ = bool(p[1]);
    let p2_ = bool(p[2]);
    let p3_ = bool(p[3]);

    let pf_bits = cond_i32(pf0, 1, 0) | cond_i32(pf1, 2, 0) | cond_i32(pf2, 4, 0) | cond_i32(pf3, 8, 0);

    let p0pf = cond_i32(p0_, pf_bits, 0);
    let p1pf = cond_i32(p1_, pf_bits << 4u, 0);
    let p2pf = cond_i32(p2_, pf_bits << 8u, 0);
    let p3pf = cond_i32(p3_, pf_bits << 12u, 0);

    let m0pf = cond_i32(m0, pf_bits, 0);
    let m1pf = cond_i32(m1, pf_bits << 4u, 0);
    let m2pf = cond_i32(m2, pf_bits << 8u, 0);
    let m3pf = cond_i32(m3, pf_bits << 12u, 0);

    let player_bits = i32(p0_) | (i32(p1_) << 1u) | (i32(p2_) << 2u) | (i32(p3_) << 3u);

    let m0pl = cond_i32(m0, player_bits, 0);
    let m1pl = cond_i32(m1, player_bits << 4u, 0);
    let m2pl = cond_i32(m2, player_bits << 8u, 0);
    let m3pl = cond_i32(m3, player_bits << 12u, 0);

    let p0pl = cond_i32(p0_, player_bits & ~1, 0);
    let p1pl = cond_i32(p1_, (player_bits & ~2) << 4u, 0);
    let p2pl = cond_i32(p2_, (player_bits & ~4) << 8u, 0);
    let p3pl = cond_i32(p3_, (player_bits & ~8) << 12u, 0);

    let o_CollisionsTarget = vec4<u32>(
        u32(m0pf | m1pf | m2pf | m3pf),
        u32(p0pf | p1pf | p2pf | p3pf),
        u32(m0pl | m1pl | m2pl | m3pl),
        u32(p0pl | p1pl | p2pl | p3pl),
    );
    var out_color = palette.palette[color_reg];
    if(scan_line == antic_config.debug_scan_line) {
        let alpha = 0.5;
        out_color = vec4<f32>(alpha * vec3<f32>(1.0, 0.0, 0.0) + (1.0 - alpha) * out_color.rgb, 1.0);
    };

    return FragmentOutput(out_color, o_CollisionsTarget);
}
