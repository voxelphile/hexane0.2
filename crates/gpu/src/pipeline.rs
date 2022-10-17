use crate::prelude::*;

use std::borrow;
use std::env;
use std::path;
use std::process;

pub type Spv = Vec<u32>;

///Requires a name, a type, and the includes
#[derive(Clone)]
pub struct Shader<'a>(&'a str, ShaderType, &'a [&'a str]);

#[derive(Default)]
pub enum ShaderLanguage {
    #[default]
    Glsl,
    Hlsl,
}

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

pub struct PiplineCompilerInfo<'a> {
    compiler: ShaderCompiler,
    source_path: &'a path::Path,
    output_path: &'a path::Path,
    debug_name: &'a str,
}

impl Default for PipelineCompilerInfo {
    fn default() -> Self {
        Self {
            compiler: Default::default(),
            source_path: exe::current_dir().expect("failed to get current directory"),
            output_path: exe::current_dir().expect("failed to get current directory"),
            debug_name: "PipelineCompiler",
        }
    }
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
    pub fn glslc(info: ShaderCompilerInfo) -> Result<Self> {
        let ShaderCompilerInfo { language } = info;

        ShaderCompiler::Glslc { language }
    }

    pub fn dxc(info: ShaderCompilerInfo) -> Result<Self> {
        let ShaderCompilerInfo { language } = info;

        ShaderCompiler::Dxc { language }
    }

    pub(crate) fn compile_to_spv(&self, options: ShaderCompilationOptions) -> Result<Spv> {
        let code = vec![];

        let vulkan_path = env::var("VULKAN_SDK").map_err(Error::ShaderCompilerNotFound)?;

        match self {
            ShaderCompiler::Glslc { language } => {
                let glslc_path = vulkan_path.join("Bin").with_file_name("glslc.exe");

                let glslc = Command::new(&glslc_path)
                    .arg("-O")
                    .arg(format!(
                        "-fshader-stage={}",
                        match &options.ty {
                            Vertex => "vertex",
                            Fragment => "fragment",
                            Compute => "compute",
                        }
                    ))
                    .arg(format!("-c {}", &options.input_path))
                    .arg(&options.output_path)
                    .spawn()
                    .map_err(Error::ShaderCompilerNotFound)?;

                let glslc = glslc.wait_with_output();

                let spv = glslc
                    .exit_ok()
                    .map(|_| fs::read_to_string(options.output_path))
                    .map_err(|_| Error::ShaderCompilationError {
                        message: String::from_utf8(glslc.std_err).expect("could not part glslc std err"),
                    })
                    .flatten();

                fs::remove_file(options.output_path);

                spv
            }
            _ => todo!(),
        }

        Ok(code)
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
    pub fn create_pipeline(info: PipelineInfo<'_>) -> Result<Pipeline> {
        todo!()
    }

    pub fn refresh(&self, pipeline: &mut Pipeline) {}
}

pub enum PipelineInfo<'a> {
    Graphics {
        shaders: &[Shader],
        attachments: &[Attachment],
        raster: RasterizerInfo,
        depth: Option<DepthInfo>,
        push_constant_size: usize,
        debug_name: &'a str,
    },
    Compute {
        shader: Shader,
        push_constant_size: usize,
        debug_name: &'a str,
    },
}

impl Default for PipelineInfo<'_> {
    fn default() -> Self {
        Self::Graphics {
            shaders: &[],
            attachments: &[],
            raster: default(),
            depth: None,
            push_constant_size: 0,
            debug_name: "Pipeline",
        }
    }
}

pub struct Pipeline {
    pipeline: vk::Pipeline,
    spec: PipelineSpec,
}
