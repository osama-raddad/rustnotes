//! Immediate-mode OpenGL ES 3 renderer: colored quads + a fontdue-powered glyph
//! atlas. One VBO, two tiny shader programs, everything rebuilt per frame (fine
//! for a UI this small).

use crate::egl::Egl;
use crate::text::FontBook;
use glow::*;
use std::collections::HashMap;

pub type Color = [f32; 4];

pub const SCREEN_BG: Color = [0.965, 0.965, 0.973, 1.0];

const VS_SOLID: &str = r#"#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
layout(location=1) in vec4 a_color;
uniform vec2 u_res;
out vec4 v_color;
void main() {
    vec2 clip = vec2(a_pos.x / u_res.x * 2.0 - 1.0, 1.0 - a_pos.y / u_res.y * 2.0);
    gl_Position = vec4(clip, 0.0, 1.0);
    v_color = a_color;
}"#;

const FS_SOLID: &str = r#"#version 300 es
precision mediump float;
in vec4 v_color;
out vec4 o;
void main() { o = v_color; }"#;

const VS_TEXT: &str = r#"#version 300 es
precision highp float;
layout(location=0) in vec2 a_pos;
layout(location=1) in vec2 a_uv;
layout(location=2) in vec4 a_color;
uniform vec2 u_res;
out vec2 v_uv;
out vec4 v_color;
void main() {
    vec2 clip = vec2(a_pos.x / u_res.x * 2.0 - 1.0, 1.0 - a_pos.y / u_res.y * 2.0);
    gl_Position = vec4(clip, 0.0, 1.0);
    v_uv = a_uv;
    v_color = a_color;
}"#;

const FS_TEXT: &str = r#"#version 300 es
precision mediump float;
in vec2 v_uv;
in vec4 v_color;
uniform sampler2D u_atlas;
out vec4 o;
void main() { o = vec4(v_color.rgb, v_color.a * texture(u_atlas, v_uv).r); }"#;

struct GlyphQuad {
    uv: [f32; 4], // u0, v0, u1, v1 (v0 = top)
    x_off: f32,   // left bearing
    y_top: f32,   // bitmap top relative to baseline (negative = above)
    w: f32,
    h: f32,
    advance: f32,
}

const ATLAS_DIM: u32 = 512;

pub struct Renderer {
    gl: Context,
    egl: Egl,
    solid: NativeProgram,
    solid_res: Option<NativeUniformLocation>,
    text: NativeProgram,
    text_res: Option<NativeUniformLocation>,
    text_atlas: Option<NativeUniformLocation>,
    vao: NativeVertexArray,
    vbo: NativeBuffer,
    atlas: NativeTexture,
    pen: (u32, u32),
    row_h: u32,
    glyphs: HashMap<(char, u32), GlyphQuad>,
    solid_verts: Vec<f32>,
    text_verts: Vec<f32>,
    pub fonts: FontBook,
    pub w: i32,
    pub h: i32,
}

fn f32_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr().cast(), std::mem::size_of_val(v)) }
}

fn compile(gl: &Context, ty: u32, src: &str) -> Result<NativeShader, String> {
    unsafe {
        let sh = gl.create_shader(ty).map_err(|_| "create_shader".to_string())?;
        gl.shader_source(sh, src);
        gl.compile_shader(sh);
        if !gl.get_shader_compile_status(sh) {
            let log = gl.get_shader_info_log(sh);
            gl.delete_shader(sh);
            return Err(format!("shader compile: {log}"));
        }
        Ok(sh)
    }
}

fn link(gl: &Context, vs: &str, fs: &str) -> Result<NativeProgram, String> {
    unsafe {
        let v = compile(gl, VERTEX_SHADER, vs)?;
        let f = compile(gl, FRAGMENT_SHADER, fs)?;
        let p = gl.create_program().map_err(|_| "create_program".to_string())?;
        gl.attach_shader(p, v);
        gl.attach_shader(p, f);
        gl.link_program(p);
        gl.delete_shader(v);
        gl.delete_shader(f);
        if !gl.get_program_link_status(p) {
            let log = gl.get_program_info_log(p);
            gl.delete_program(p);
            return Err(format!("program link: {log}"));
        }
        Ok(p)
    }
}

impl Renderer {    pub fn new(window: *mut std::ffi::c_void) -> Result<Renderer, String> {
        let egl = Egl::new(window)?;
        let gl =
            unsafe { Context::from_loader_function(|n| egl.get_proc_address(n) as *const _) };

        unsafe {
            gl.enable(BLEND);
            gl.blend_func(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
            gl.disable(DEPTH_TEST);
            gl.pixel_store_i32(UNPACK_ALIGNMENT, 1);
        }

        let solid = link(&gl, VS_SOLID, FS_SOLID)?;
        let text = link(&gl, VS_TEXT, FS_TEXT)?;
        let solid_res = unsafe { gl.get_uniform_location(solid, "u_res") };
        let text_res = unsafe { gl.get_uniform_location(text, "u_res") };
        let text_atlas = unsafe { gl.get_uniform_location(text, "u_atlas") };

        let vao = unsafe { gl.create_vertex_array() }.map_err(|_| "vao".to_string())?;
        let vbo = unsafe { gl.create_buffer() }.map_err(|_| "vbo".to_string())?;

        let atlas = unsafe { gl.create_texture() }.map_err(|_| "texture".to_string())?;
        unsafe {
            gl.bind_texture(TEXTURE_2D, Some(atlas));
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                R8 as i32,
                ATLAS_DIM as i32,
                ATLAS_DIM as i32,
                0,
                RED,
                UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, LINEAR as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);
        }

        let fonts = FontBook::load()?;
        let (w, h) = egl.surface_size();

        let mut r = Renderer {
            gl,
            egl,
            solid,
            solid_res,
            text,
            text_res,
            text_atlas,
            vao,
            vbo,
            atlas,
            pen: (0, 0),
            row_h: 0,
            glyphs: HashMap::new(),
            solid_verts: Vec::with_capacity(4096),
            text_verts: Vec::with_capacity(16384),
            fonts,
            w,
            h,
        };
        r.resize();
        Ok(r)
    }

    pub fn resize(&mut self) {
        let (w, h) = self.egl.surface_size();
        if w > 0 && h > 0 {
            self.w = w;
            self.h = h;
        }
        unsafe { self.gl.viewport(0, 0, self.w, self.h) };
    }

    pub fn destroy(&mut self) {
        unsafe {
            self.gl.delete_program(self.solid);
            self.gl.delete_program(self.text);
            self.gl.delete_buffer(self.vbo);
            self.gl.delete_vertex_array(self.vao);
            self.gl.delete_texture(self.atlas);
        }
        self.egl.destroy();
    }

    // ---- glyph atlas ----------------------------------------------------

    fn alloc_rect(&mut self, w: u32, h: u32) -> [f32; 4] {
        let d = ATLAS_DIM;
        let pad = 1u32;
        if self.pen.0 + w + pad > d {
            self.pen.0 = 0;
            self.pen.1 += self.row_h + pad;
            self.row_h = 0;
        }
        if self.pen.1 + h + pad > d {
            // Atlas full: wipe and start over (cache repopulates on demand).
            unsafe {
                self.gl.bind_texture(TEXTURE_2D, Some(self.atlas));
                let zeros = vec![0u8; (d * d) as usize];
                self.gl.tex_sub_image_2d(
                    TEXTURE_2D,
                    0,
                    0,
                    0,
                    d as i32,
                    d as i32,
                    RED,
                    UNSIGNED_BYTE,
                    PixelUnpackData::Slice(&zeros[..])
                );
            }
            self.glyphs.clear();
            self.pen = (0, 0);
            self.row_h = 0;
        }
        let (x, y) = self.pen;
        self.pen.0 += w + pad;
        self.row_h = self.row_h.max(h);
        [
            x as f32 / d as f32,
            y as f32 / d as f32,
            (x + w) as f32 / d as f32,
            (y + h) as f32 / d as f32,
        ]
    }

    fn glyph(&mut self, ch: char, pxu: u32) -> &GlyphQuad {
        if !self.glyphs.contains_key(&(ch, pxu)) {
            let (metrics, bitmap) = self.fonts.rasterize(ch, pxu as f32);
            let (w, h) = (metrics.width.max(1) as u32, metrics.height.max(1) as u32);
            let uv = self.alloc_rect(w, h);
            if metrics.width > 0 && metrics.height > 0 {
                let (ux, vy) = (
                    (uv[0] * ATLAS_DIM as f32) as i32,
                    (uv[1] * ATLAS_DIM as f32) as i32,
                );
                unsafe {
                    self.gl.bind_texture(TEXTURE_2D, Some(self.atlas));
                    self.gl.tex_sub_image_2d(
                        TEXTURE_2D,
                        0,
                        ux,
                        vy,
                        metrics.width as i32,
                        metrics.height as i32,
                        RED,
                        UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(&bitmap[..]),
                    );
                }
            }
            let g = GlyphQuad {
                uv,
                x_off: metrics.xmin as f32,
                y_top: -(metrics.ymin as f32),
                w: metrics.width as f32,
                h: metrics.height as f32,
                advance: metrics.advance_width,
            };
            self.glyphs.insert((ch, pxu), g);
        }
        self.glyphs.get(&(ch, pxu)).unwrap()
    }

    // ---- drawing ---------------------------------------------------------

    pub fn begin(&mut self) {
        self.solid_verts.clear();
        self.text_verts.clear();
    }

    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        let vs = &mut self.solid_verts;
        let (x0, y0, x1, y1) = (x, y, x + w, y + h);
        for (px, py) in [
            (x0, y0),
            (x1, y0),
            (x0, y1),
            (x1, y0),
            (x1, y1),
            (x0, y1),
        ] {
            vs.extend_from_slice(&[px, py, c[0], c[1], c[2], c[3]]);
        }
    }

    pub fn fill_circle(&mut self, cx: f32, cy: f32, r: f32, c: Color) {
        const N: usize = 20;
        for i in 0..N {
            let a0 = (i as f32) * std::f32::consts::TAU / N as f32;
            let a1 = ((i + 1) as f32) * std::f32::consts::TAU / N as f32;
            let p0 = (cx + a0.cos() * r, cy + a0.sin() * r);
            let p1 = (cx + a1.cos() * r, cy + a1.sin() * r);
            self.tri((cx, cy), p0, p1, c);
        }
    }

    pub fn fill_round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, c: Color) {
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        let segs = 4;
        let corners = [
            (x + r, y + r, std::f32::consts::PI),
            (x + w - r, y + r, 1.5 * std::f32::consts::PI),
            (x + w - r, y + h - r, 0.0),
            (x + r, y + h - r, 0.5 * std::f32::consts::PI),
        ];
        let mut pts: Vec<(f32, f32)> = Vec::with_capacity(corners.len() * (segs + 1));
        for (ccx, ccy, a0) in corners {
            for i in 0..=segs {
                let a = a0 + (i as f32 / segs as f32) * (std::f32::consts::FRAC_PI_2);
                pts.push((ccx + a.cos() * r, ccy + a.sin() * r));
            }
        }
        let c0 = (x + w / 2.0, y + h / 2.0);
        for i in 0..pts.len() {
            self.tri(c0, pts[i], pts[(i + 1) % pts.len()], c);
        }
    }

    pub fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, c: Color) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt().max(0.0001);
        let nx = -dy / len * width / 2.0;
        let ny = dx / len * width / 2.0;
        let a = (x0 + nx, y0 + ny);
        let b = (x1 + nx, y1 + ny);
        let d = (x1 - nx, y1 - ny);
        let e = (x0 - nx, y0 - ny);
        self.tri(a, b, d, c);
        self.tri(a, d, e, c);
    }

    fn tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Color) {
        let vs = &mut self.solid_verts;
        for (px, py) in [a, b, c] {
            vs.extend_from_slice(&[px, py, col[0], col[1], col[2], col[3]]);
        }
    }

    fn push_glyph(&mut self, x: f32, y: f32, g: &GlyphQuad, c: Color) {
        if g.w <= 0.0 || g.h <= 0.0 {
            return;
        }
        let (u0, v0, u1, v1) = (g.uv[0], g.uv[1], g.uv[2], g.uv[3]);
        let (x1, y1) = (x + g.w, y + g.h);
        let vs = &mut self.text_verts;
        let q = [
            (x, y, u0, v0),
            (x1, y, u1, v0),
            (x, y1, u0, v1),
            (x1, y, u1, v0),
            (x1, y1, u1, v1),
            (x, y1, u0, v1),
        ];
        for (px, py, u, v) in q {
            vs.extend_from_slice(&[px, py, u, v, c[0], c[1], c[2], c[3]]);
        }
    }

    /// Draw chars[start..end] with `baseline` at the baseline. Returns width drawn.
    pub fn text_run(
        &mut self,
        x: f32,
        baseline: f32,
        chars: &[char],
        start: usize,
        end: usize,
        px: f32,
        c: Color,
    ) -> f32 {
        let pxu = px.max(4.0) as u32;
        let mut pen = x;
        for i in start..end {
            let ch = chars[i];
            let (x_off, y_top, w, h, adv, uv) = {
                let g = self.glyph(ch, pxu);
                (g.x_off, g.y_top, g.w, g.h, g.advance, g.uv)
            };
            let gg = GlyphQuad {
                uv,
                x_off,
                y_top,
                w,
                h,
                advance: adv,
            };
            self.push_glyph(pen + x_off, baseline + y_top, &gg, c);
            pen += adv;
            if i + 1 < end {
                pen += self.fonts.kern(ch, chars[i + 1], px);
            }
        }
        pen - x
    }

    pub fn text_str(&mut self, x: f32, baseline: f32, s: &str, px: f32, c: Color) -> f32 {
        let chars: Vec<char> = s.chars().collect();
        self.text_run(x, baseline, &chars, 0, chars.len(), px, c)
    }

    /// Advance-only measurement (no rasterization).
    pub fn text_width(&mut self, s: &str, px: f32) -> f32 {
        s.chars().map(|ch| self.fonts.advance(ch, px)).sum()
    }

    /// Single line, truncated with "..." if wider than max_w.
    pub fn text_str_clipped(&mut self, x: f32, baseline: f32, s: &str, max_w: f32, px: f32, c: Color) {
        if max_w <= 4.0 || s.is_empty() {
            return;
        }
        if self.text_width(s, px) <= max_w {
            self.text_str(x, baseline, s, px, c);
            return;
        }
        let ell = self.text_width("...", px);
        let limit = max_w - ell;
        let mut trunc = String::new();
        let mut pen = 0.0f32;
        for ch in s.chars() {
            let adv = self.fonts.advance(ch, px);
            if pen + adv > limit {
                break;
            }
            trunc.push(ch);
            pen += adv;
        }
        trunc.push_str("...");
        self.text_str(x, baseline, &trunc, px, c);
    }

    pub fn finish(&mut self) {
        let gl = &self.gl;
        unsafe {
            gl.viewport(0, 0, self.w, self.h);
            gl.clear_color(0.965, 0.965, 0.973, 1.0);
            gl.clear(COLOR_BUFFER_BIT);

            gl.use_program(Some(self.solid));
            gl.uniform_2_f32(
                self.solid_res.as_ref(),
                self.w as f32,
                self.h as f32,
            );
            gl.bind_vertex_array(Some(self.vao));
            gl.bind_buffer(ARRAY_BUFFER, Some(self.vbo));
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 24, 0);
            gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, 24, 8);
            gl.enable_vertex_attrib_array(0);
            gl.enable_vertex_attrib_array(1);
            gl.disable_vertex_attrib_array(2);
            if !self.solid_verts.is_empty() {
                gl.buffer_data_u8_slice(ARRAY_BUFFER, f32_bytes(&self.solid_verts), STREAM_DRAW);
                gl.draw_arrays(TRIANGLES, 0, (self.solid_verts.len() / 6) as i32);
            }

            gl.use_program(Some(self.text));
            gl.uniform_2_f32(self.text_res.as_ref(), self.w as f32, self.h as f32);
            gl.uniform_1_i32(self.text_atlas.as_ref(), 0);
            gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 32, 0);
            gl.vertex_attrib_pointer_f32(1, 2, FLOAT, false, 32, 8);
            gl.vertex_attrib_pointer_f32(2, 4, FLOAT, false, 32, 16);
            gl.enable_vertex_attrib_array(0);
            gl.enable_vertex_attrib_array(1);
            gl.enable_vertex_attrib_array(2);
            gl.active_texture(TEXTURE0);
            gl.bind_texture(TEXTURE_2D, Some(self.atlas));
            if !self.text_verts.is_empty() {
                gl.buffer_data_u8_slice(ARRAY_BUFFER, f32_bytes(&self.text_verts), STREAM_DRAW);
                gl.draw_arrays(TRIANGLES, 0, (self.text_verts.len() / 8) as i32);
            }
            gl.bind_vertex_array(None);
        }
        self.egl.swap();
    }
}
