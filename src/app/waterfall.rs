use eframe::glow::{
    self, PixelUnpackData, CLAMP_TO_EDGE, TEXTURE0, TEXTURE1, TEXTURE_WRAP_S, UNSIGNED_BYTE,
};
use glow::HasContext as _;
use glow::{NEAREST, TEXTURE_2D, TEXTURE_MAG_FILTER, TEXTURE_MIN_FILTER};
use log;
use std::mem::{size_of, transmute};
use std::sync::mpsc::Receiver;

const SIZE_OF_F32: i32 = size_of::<f32>() as i32;

unsafe fn check_for_gl_errors(gl: &glow::Context, msg: &str) {
    while let Some(err) = match gl.get_error() {
        glow::NO_ERROR => None,
        err => Some(err),
    } {
        log::error!("Waterfall {}: GL ERROR {} ({:#X})", msg, err, err);
    }
}

use crate::app::turbo_colormap;

pub struct Waterfall {
    program: glow::Program,
    texture: glow::Texture,
    color_lut: glow::Texture,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    offset: usize,
    width: usize,
    fft_in: Receiver<Vec<u8>>,
}

impl Waterfall {
    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_texture(self.texture);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ebo);
            check_for_gl_errors(&gl, "APP CLOSE");
        }
    }
    pub fn paint(&mut self, gl: &glow::Context, _angle: f32) {
        use glow::HasContext as _;

        unsafe {
            // Bind our texturs
            gl.active_texture(TEXTURE1);
            check_for_gl_errors(&gl, "Active texture 1");
            gl.bind_texture(glow::TEXTURE_2D, Some(self.color_lut));
            check_for_gl_errors(&gl, "bind lut");

            gl.active_texture(TEXTURE0);
            check_for_gl_errors(&gl, "Active texture 0");
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            check_for_gl_errors(&gl, "bind texture");

            // Use our shader program
            gl.use_program(Some(self.program));
            check_for_gl_errors(&gl, "use program");

            // Bind our vertex array object
            gl.bind_vertex_array(Some(self.vao));
            check_for_gl_errors(&gl, "bind vao");

            // Update texture
            while let Ok(fft) = self.fft_in.try_recv() {
                if fft.len() != self.width {
                    todo!();
                }
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    self.offset as i32,
                    self.width as i32,
                    1,
                    glow::RED,
                    glow::UNSIGNED_BYTE,
                    PixelUnpackData::Slice(&fft),
                );
                check_for_gl_errors(&gl, "update texture");
                self.offset = (self.offset + 1) % self.width;
            }

            if let Some(uniform) = gl.get_uniform_location(self.program, "offset") {
                gl.uniform_1_f32(Some(&uniform), self.offset as f32 / self.width as f32);
            }
            check_for_gl_errors(&gl, "update uniform");

            // Draw the elements
            gl.draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_INT, 0);

            // Log and clear the error queue of any errors
            check_for_gl_errors(&gl, "APP PAINT");
        }
    }
    pub fn new(gl: &glow::Context, width: usize, height: usize, fft_in: Receiver<Vec<u8>>) -> Self {
        let vertices: [f32; 32] = [
            // positions      // colors          // texture coords
            1.0, 1.0, 0.0, /**/ 1.0, 0.0, 0.0, /**/ 1.0, 1.0, // top right
            1.0, -1.0, 0.0, /**/ 0.0, 1.0, 0.0, /**/ 1.0, 0.0, // bottom right
            -1.0, -1.0, 0.0, /**/ 0.0, 0.0, 1.0, /**/ 0.0, 0.0, // bottom left
            -1.0, 1.0, 0.0, /**/ 1.0, 1.0, 0.0, /**/ 0.0, 1.0, // top left
        ];
        let indices: [i32; 6] = [
            0, 1, 3, // First triangle
            1, 2, 3,
        ];
        let shader_version: &str = if cfg!(target_arch = "wasm32") || cfg!(target_os = "android") {
            "#version 300 es"
        } else {
            "#version 330"
        };

        // Generate something to put into the texture Buffer
        let mut buffer = vec![0; width * height];
        // Add some stripes to the texture
        let stripes = 8;
        let slen = width / stripes;
        for (i, val) in buffer.iter_mut().enumerate() {
            *val = if i % slen < slen / 2 { 255 } else { 0 };
            //*val = 255;
        }

        unsafe {
            let vao = gl
                .create_vertex_array()
                .expect("Could not create vertex array");
            let vbo = gl.create_buffer().expect("Could not create vertex buffer");
            let ebo = gl.create_buffer().expect("Could not create element buffer");
            check_for_gl_errors(&gl, "Create buffers");

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &transmute::<[f32; 32], [u8; 128]>(vertices),
                glow::STATIC_DRAW,
            );

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                &transmute::<[i32; 6], [u8; 24]>(indices),
                glow::STATIC_DRAW,
            );

            // Position attribute
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 8 * SIZE_OF_F32, 0);
            gl.enable_vertex_attrib_array(0);
            // Color attribute
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                8 * SIZE_OF_F32,
                3 * SIZE_OF_F32,
            );
            gl.enable_vertex_attrib_array(1);
            // Position attribute
            gl.vertex_attrib_pointer_f32(
                2,
                2,
                glow::FLOAT,
                false,
                8 * SIZE_OF_F32,
                6 * SIZE_OF_F32,
            );
            gl.enable_vertex_attrib_array(2);

            // Texture
            let texture = gl
                .create_texture()
                .expect("Waterfall: Could not create texture");

            gl.bind_texture(glow::TEXTURE_2D, Some(texture));

            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
            check_for_gl_errors(&gl, "Set texture params");

            //gl.tex_storage_2d(glow::TEXTURE_2D, 1, glow::R8, 300, 300);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::R8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RED,
                glow::UNSIGNED_BYTE,
                Some(&buffer),
            );
            check_for_gl_errors(&gl, "Initializing Texture");

            let color_lut = gl
                .create_texture()
                .expect("Waterfall: could not create LUT");
            gl.bind_texture(TEXTURE_2D, Some(color_lut));
            check_for_gl_errors(&gl, "Setup Bind LUT");
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MIN_FILTER, NEAREST as i32);
            check_for_gl_errors(&gl, "Set LUT MIN_FILTER");
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_MAG_FILTER, NEAREST as i32);
            check_for_gl_errors(&gl, "Set LUT MAG_FILTER");
            gl.tex_parameter_i32(TEXTURE_2D, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
            check_for_gl_errors(&gl, "Set LUT wrap mode");
            gl.tex_image_2d(
                TEXTURE_2D,
                0,
                if cfg!(target_os = "android") || cfg!(target_arch = "wasm32") {
                    glow::RGB
                } else {
                    glow::SRGB
                } as i32,
                256,
                1,
                0,
                glow::RGB,
                UNSIGNED_BYTE,
                Some(&turbo_colormap::TURBO_SRGB_BYTES),
            );
            check_for_gl_errors(&gl, "Initializing LUT");

            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
                r#"
                    layout (location = 0) in vec3 aPos;
                    layout (location = 1) in vec3 aColor;
                    layout (location = 2) in vec3 aTexCoord;

                    out vec3 ourColor;
                    out vec2 TexCoord;

                    void main()
                    {
                        gl_Position = vec4(aPos, 1.0);
                        ourColor = aColor;
                        TexCoord = vec2(aTexCoord.x, aTexCoord.y);
                    }
                "#,
                r#"
                    precision mediump float;

                    out vec4 FragColor;

                    in vec3 ourColor;
                    in vec2 TexCoord;

                    // texture sampler
                    uniform sampler2D texture1;
                    uniform sampler2D LUT;
                    uniform float offset;

                    void main()
                    {
                        float val = texture(texture1, vec2(TexCoord.x, TexCoord.y + offset)).x;
                        FragColor = texture(LUT, vec2(val, 0));
                    }
                "#,
            );

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Waterfall: Cannot create shader");
                    gl.shader_source(shader, &format!("{shader_version}\n{shader_source}"));
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Waterfall Failed to compile {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );
                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();
            check_for_gl_errors(&gl, "Compiling shaders");

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );
            check_for_gl_errors(&gl, "Link GL Program");

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            gl.use_program(Some(program));
            gl.uniform_1_i32(gl.get_uniform_location(program, "texture1").as_ref(), 0);
            gl.uniform_1_i32(gl.get_uniform_location(program, "LUT").as_ref(), 1);
            check_for_gl_errors(&gl, "APP INIT");

            Self {
                program,
                texture,
                color_lut,
                vao,
                vbo,
                ebo,
                offset: 0_usize,
                width,
                fft_in,
            }
        }
    }
}
