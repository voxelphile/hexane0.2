use crate::prelude::*;

use std::borrow;
use std::default::default;
use std::env;
use std::fs;
use std::path;
use std::process;

use ash::vk;

use bitflags::bitflags;

use lazy_static::lazy_static;

pub type Spv = Vec<u32>;

///Requires a name, a type, and the includes
#[derive(Clone)]
pub struct Shader<'a>(pub ShaderType, pub &'a str, pub &'a [&'a str]);

#[derive(Default)]
pub enum ShaderLanguage {
    #[default]
    Glsl,
    Hlsl,
}

#[derive(Clone)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Default)]
pub struct ShaderCompilerInfo {
    language: ShaderLanguage,
}

pub struct ShaderCompilationOptions<'a> {
    input_path: &'a path::Path,
    output_path: &'a path::Path,
    ty: ShaderType,
}

pub enum ShaderCompiler {
    None,
    Glslc { language: ShaderLanguage },
    Dxc { language: ShaderLanguage },
}

impl Default for ShaderCompiler {
    fn default() -> Self {
        Self::None
    }
}

impl ShaderCompiler {
    pub fn glslc(info: ShaderCompilerInfo) -> Self {
        let ShaderCompilerInfo { language } = info;

        ShaderCompiler::Glslc { language }
    }

    pub fn dxc(info: ShaderCompilerInfo) -> Self {
        let ShaderCompilerInfo { language } = info;

        ShaderCompiler::Dxc { language }
    }

    pub(crate) fn compile_to_spv(&self, options: ShaderCompilationOptions) -> Result<Spv> {
        let vulkan_path = env::var("VULKAN_SDK").map_err(|_| Error::ShaderCompilerNotFound)?;

        match self {
            ShaderCompiler::Glslc { language } => {
                let glslc_path = path::PathBuf::from(vulkan_path)
                    .join("Bin")
                    .with_file_name("glslc.exe");

                let glslc = process::Command::new(&glslc_path)
                    .arg("-O")
                    .arg(format!(
                        "-fshader-stage={}",
                        match &options.ty {
                            Vertex => "vertex",
                            Fragment => "fragment",
                            Compute => "compute",
                        }
                    ))
                    .arg(format!("-c {}", &options.input_path.display()))
                    .arg(&options.output_path)
                    .spawn()
                    .map_err(|_| Error::ShaderCompilerNotFound)?;

                let glslc = glslc
                    .wait_with_output()
                    .map_err(|_| Error::ShaderCompilerNotFound)?;

                let spv = glslc
                    .status
                    .exit_ok()
                    .map(|_| {
                        fs::read(options.output_path)
                            .expect("failed to read shader compilation output")
                            .chunks(4)
                            .map(|a| u32::from_le_bytes(a.try_into().unwrap()))
                            .collect::<Vec<_>>()
                    })
                    .map_err(|_| Error::ShaderCompilationError {
                        message: String::from_utf8(glslc.stderr)
                            .expect("could not part glslc std err"),
                    })?;

                fs::remove_file(options.output_path);

                Ok(spv)
            }
            _ => todo!(),
        }
    }
}

#[cfg(target_os = "windows")]
lazy_static! {
    static ref CURRENT_PATH: path::PathBuf =
        env::current_dir().expect("failed to get current directory");
}

pub struct PipelineCompilerInfo<'a> {
    pub compiler: ShaderCompiler,
    pub source_path: &'a path::Path,
    pub output_path: &'a path::Path,
    pub debug_name: &'a str,
}

impl Default for PipelineCompilerInfo<'_> {
    fn default() -> Self {
        Self {
            compiler: Default::default(),
            source_path: &CURRENT_PATH,
            output_path: &CURRENT_PATH,
            debug_name: "PipelineCompiler",
        }
    }
}

pub struct PipelineCompiler {
    pipelines: Vec<Pipeline>,
    compiler: ShaderCompiler,
    source_path: path::PathBuf,
    output_path: path::PathBuf,
    debug_name: String,
}

impl PipelineCompiler {
    pub fn create_graphics_pipeline(&self, info: GraphicsPipelineInfo<'_>) -> Result<Pipeline> {
        todo!()
    }

    pub fn create_compute_pipeline(&self, info: ComputePipelineInfo<'_>) -> Result<Pipeline> {
        todo!()
    }

    pub fn refresh(&self, pipeline: &mut Pipeline) {}
}

#[derive(Default)]
pub enum PolygonMode {
    #[default]
    Fill,
    Line,
    Point,
}

bitflags! {
    #[derive(Default)]
    pub struct FaceCull : u32 {
        const NONE = 0x00000000;
        const FRONT = 0x00000002;
        const BACK = 0x00000004;
        const FRONT_AND_BACK = Self::FRONT.bits | Self::BACK.bits;
    }
}

bitflags! {
    pub struct ColorComponent : u32 {
        const R = 0x00000002;
        const G = 0x00000004;
        const B = 0x00000008;
        const A = 0x00000020;
        const ALL = Self::R.bits
                                | Self::G.bits
                                | Self::B.bits
                                | Self::A.bits;
    }
}

pub struct Raster {
    pub polygon_mode: PolygonMode, pub
    face_cull: FaceCull, pub
    depth_clamp: bool, pub
    rasterizer_discard: bool, pub
    depth_bias: bool, pub
    depth_bias_constant_factor: f32, pub
    depth_bias_clamp: f32, pub
    depth_bias_slope_factor: f32, pub
    line_width: f32,
}

impl Default for Raster {
    fn default() -> Self {
        Self {
            polygon_mode: default(),
            face_cull: default(),
            depth_clamp: default(),
            rasterizer_discard: default(),
            depth_bias: default(),
            depth_bias_constant_factor: default(),
            depth_bias_clamp: default(),
            depth_bias_slope_factor: default(),
            line_width: 1.0,
        }
    }
}

pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}

pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

pub struct Blend {
    pub src_color: BlendFactor,
    pub dst_color: BlendFactor,
    pub color_blend: BlendOp,
    pub src_alpha: BlendFactor,
    pub dst_alpha: BlendFactor,
    pub alpha_blend: BlendOp,
    pub color_write: ColorComponent,
}

impl Default for Blend {
    fn default() -> Self {
        Self {
            src_color: BlendFactor::One,
            dst_color: BlendFactor::Zero,
            color_blend: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::Zero,
            alpha_blend: BlendOp::Add,
            color_write: ColorComponent::ALL,
        }
    }
}

#[derive(Default)]
pub struct Color {
    pub format: Format,
    pub blend: Option<Blend>,
}

#[derive(Default)]
pub enum CompareOp {
    #[default]
    Never,
    Less,
    Equal,
    LessOrEqual,
    Greater,
    NotEqual,
    GreaterOrEqual,
    Always,
}

pub struct Depth {
    pub write: bool,
    pub compare: CompareOp,
    pub bounds: (f32, f32),
}

impl Default for Depth {
    fn default() -> Self {
        Self {
            write: default(),
            compare: default(),
            bounds: (0.0, 1.0),
        }
    }
}

pub struct GraphicsPipelineInfo<'a> {
    pub shaders: &'a [Shader<'a>],
    pub color: &'a [Color], pub
    depth: Option<Depth>, pub
    raster: Raster, pub
    push_constant_size: usize, pub
    debug_name: &'a str, 
}

impl Default for GraphicsPipelineInfo<'_> {
    fn default() -> Self {
        Self {
            shaders: &[],
            color: &[],
            depth: None,
            raster: default(),
            push_constant_size: 0,
            debug_name: "Pipeline",
        }
    }
}

pub struct ComputePipelineInfo<'a> {
    pub shader: Shader<'a>, pub
    push_constant_size: usize, pub
    debug_name: &'a str, 
}

impl Default for ComputePipelineInfo<'_> {
    fn default() -> Self {
        Self {
            shader: Shader(ShaderType::Compute, "default", &[]),
            push_constant_size: 0,
            debug_name: "Pipeline",
        }
    }
}

#[derive(Clone, Copy)]
pub struct Pipeline(usize);
