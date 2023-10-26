use glsl::{parser::Parse as _, syntax};
use log::{info, warn};
use std::{fs, path::Path};

use ash::vk;

use crate::error::{Error, VResult};

enum MemoryLayout {
    STD140,
    STD430,
}

#[derive(Debug)]
pub enum ImageFormat {
    RGBA32F,
}

#[derive(Default)]
struct TypeProperties {
    storage: Option<syntax::StorageQualifier>,
    binding: Option<usize>,
    set: Option<usize>,
    offset: Option<usize>,
    push_constant: bool,
    memory_layout: Option<MemoryLayout>,
    image_format: Option<ImageFormat>,
    local_size: Option<(usize, usize, usize)>,
}

fn simplify_layout_qualifier_spec(
    type_properties: &mut TypeProperties,
    local_size: &mut (usize, usize, usize),
    layout_qualifier_spec: &syntax::LayoutQualifierSpec,
) -> VResult<()> {
    use syntax::LayoutQualifierSpec as LQS;

    #[allow(clippy::match_wildcard_for_single_variants)]
    match layout_qualifier_spec {
        // Unpack layout qualifier spec, expect identifiers only.
        LQS::Identifier(name, maybe_value_box) => {
            let maybe_value = maybe_value_box
                .as_ref()
                .map(|x| match &**x {
                    syntax::Expr::IntConst(value) => Ok(usize::try_from(*value).unwrap()),
                    other => {
                        let msg = format!("Unexpected layout qualifier spec value: {other:?}");
                        Err(Error::Local(msg))
                    }
                })
                .transpose()?;

            match (name.as_str(), maybe_value) {
                // Currently we only expect int values for bindings.
                ("binding", Some(value)) => type_properties.binding = Some(value),
                ("set", Some(value)) => type_properties.set = Some(value),
                ("offset", Some(value)) => type_properties.offset = Some(value),
                ("local_size_x", Some(value)) => local_size.0 = value,
                ("local_size_y", Some(value)) => local_size.1 = value,
                ("local_size_z", Some(value)) => local_size.2 = value,
                ("rgba32f", None) => type_properties.image_format = Some(ImageFormat::RGBA32F),
                ("push_constant", None) => type_properties.push_constant = true,
                ("std140", None) => type_properties.memory_layout = Some(MemoryLayout::STD140),
                ("std430", None) => type_properties.memory_layout = Some(MemoryLayout::STD430),
                other => {
                    let msg = format!("Unexpected layout qualifier spec: {other:?}");
                    return Err(Error::Local(msg));
                }
            };
        }
        other => {
            let msg = format!("Unexpected layout qualifier spec: {other:?}");
            return Err(Error::Local(msg));
        }
    };

    Ok(())
}

fn simplify_type_qualifier(type_qualifier: &syntax::TypeQualifier) -> VResult<TypeProperties> {
    use syntax::TypeQualifierSpec as TQS;
    let syntax::TypeQualifier {
        qualifiers: syntax::NonEmpty(ref type_qualifier_specs),
    } = type_qualifier;

    let mut type_properties = TypeProperties::default();
    let mut local_size = (1, 1, 1);
    for type_qualifier_spec in type_qualifier_specs {
        match type_qualifier_spec {
            TQS::Storage(value) => type_properties.storage = Some(value.clone()),
            TQS::Layout(syntax::LayoutQualifier {
                ids: syntax::NonEmpty(ids),
            }) => ids.iter().try_for_each(|lqs| {
                simplify_layout_qualifier_spec(&mut type_properties, &mut local_size, lqs)
            })?,
            other => {
                info!("Ignoring spec {other:?}");
            }
        };
    }
    if local_size != (1, 1, 1) {
        type_properties.local_size = Some(local_size);
    }

    Ok(type_properties)
}

fn match_globals(
    type_qualifier: &syntax::TypeQualifier,
    _global_names: &[syntax::Identifier],
) -> VResult<LocalSize> {
    let type_properties = simplify_type_qualifier(type_qualifier)?;

    if type_properties.storage != Some(syntax::StorageQualifier::In) {
        let msg = format!(
            "Unexpected global storage qualifier: {:?}",
            type_properties.storage
        );
        return Err(Error::Local(msg));
    }

    // TODO assert other stuff.

    Ok(type_properties.local_size.unwrap_or((1, 1, 1)))
}

pub trait DescriptorInfo {
    fn storage(&self) -> vk::DescriptorType;
    fn set_index(&self) -> usize;
    fn binding(&self) -> VResult<usize>;
    fn name(&self) -> &str;
}

#[derive(Debug)]
pub struct VariableDeclaration {
    pub name: String,
    pub type_specifier: syntax::TypeSpecifierNonArray,
    pub binding: Option<usize>,
    pub set: Option<usize>,
    pub image_format: Option<ImageFormat>,
}

impl DescriptorInfo for VariableDeclaration {
    fn storage(&self) -> vk::DescriptorType {
        match self.type_specifier {
            syntax::TypeSpecifierNonArray::Image2D => vk::DescriptorType::STORAGE_IMAGE,
            syntax::TypeSpecifierNonArray::Sampler2D => vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            _ => {
                warn!("Assuming STORAGE_IMAGE for {:?}", self.type_specifier);
                vk::DescriptorType::STORAGE_IMAGE
            }
        }
    }

    fn set_index(&self) -> usize {
        self.set.unwrap_or_else(|| {
            warn!("Assuming set=0 for variable {}", self.name);
            0
        })
    }

    fn binding(&self) -> VResult<usize> {
        self.binding.ok_or_else(|| {
            let msg = format!("Block '{}' does not specify a binding.", self.name);
            Error::Local(msg)
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

//
// layout(rgba32f, binding = 0) uniform image2D present;
//
// name: Some(Identifier("present")),
// ty:
//     ty: Image2D
//     qualifier: [
//         layout: [
//             Identifier(Identifier("rgba32f"), None),
//             Identifier(Identifier("binding"), Some(IntConst(0))),
//         ]
//     ]
//

fn simplify_type_specifier(
    type_specifier: &syntax::TypeSpecifier,
) -> VResult<syntax::TypeSpecifierNonArray> {
    let syntax::TypeSpecifier {
        ty,
        array_specifier,
    } = type_specifier;
    if array_specifier.is_some() {
        let msg = format!("Unexpected array specifier: {array_specifier:?}");
        return Err(Error::Local(msg));
    }
    Ok(ty.clone())
}

fn match_init_declarator_list(
    init_declarator_list: &syntax::InitDeclaratorList,
) -> VResult<Option<VariableDeclaration>> {
    let syntax::InitDeclaratorList {
        head:
            syntax::SingleDeclaration {
                ty:
                    syntax::FullySpecifiedType {
                        qualifier: type_qualifier,
                        ty: type_specifier,
                    },
                name,
                array_specifier,
                initializer,
            },
        tail,
    } = init_declarator_list;

    let type_specifier = simplify_type_specifier(type_specifier)?;
    if let syntax::TypeSpecifierNonArray::Struct(syntax::StructSpecifier { name, .. }) =
        type_specifier
    {
        warn!("Struct {name:?} will be ignored");
        return Ok(None);
    }

    let Some(type_qualifier) = type_qualifier else {
        let msg = format!("Unexpected type qualifier: {type_qualifier:?}");
        return Err(Error::Local(msg));
    };

    let type_properties = simplify_type_qualifier(type_qualifier)?;
    match type_properties.storage {
        Some(syntax::StorageQualifier::Const) => return Ok(None),
        // We assume that the storage is `Uniform`.
        Some(syntax::StorageQualifier::Uniform) => {}
        other => {
            let msg = format!("Unexpected variable type qualifier spec: {other:?}");
            return Err(Error::Local(msg));
        }
    };

    let name = if let Some(name) = name {
        name.to_string()
    } else {
        let msg = format!("Unexpected variable name: {name:?}");
        return Err(Error::Local(msg));
    };

    if array_specifier.is_some() {
        warn!("Unhandled array specifier: {array_specifier:?}");
    }

    if initializer.is_some() {
        warn!("Unhandled initializer: {initializer:?}");
    }

    if !tail.is_empty() {
        let msg = format!("Unexpected tail: {tail:?}");
        return Err(Error::Local(msg));
    }

    Ok(Some(VariableDeclaration {
        name,
        type_specifier,
        binding: type_properties.binding,
        set: type_properties.set,
        image_format: type_properties.image_format,
    }))
}

#[derive(Debug)]
pub struct BlockField {
    pub name: String,
    pub type_specifier: syntax::TypeSpecifierNonArray,
    pub offset: Option<usize>,
    pub dimensions: Option<Vec<Option<usize>>>,
}

impl BlockField {
    // We will check for dimensions and then this will be None-able.
    #[allow(clippy::unnecessary_wraps)]
    fn byte_size(&self) -> Option<usize> {
        #[allow(clippy::match_same_arms)]
        Some(match &self.type_specifier {
            syntax::TypeSpecifierNonArray::Void => 1,
            syntax::TypeSpecifierNonArray::Bool => 1,
            syntax::TypeSpecifierNonArray::Int => 4,
            syntax::TypeSpecifierNonArray::UInt => 4,
            syntax::TypeSpecifierNonArray::Float => 4,
            syntax::TypeSpecifierNonArray::Double => 8,
            syntax::TypeSpecifierNonArray::Vec2 => 8,
            syntax::TypeSpecifierNonArray::Vec3 => 12,
            syntax::TypeSpecifierNonArray::Vec4 => 16,
            syntax::TypeSpecifierNonArray::IVec2 => 8,
            syntax::TypeSpecifierNonArray::IVec3 => 12,
            syntax::TypeSpecifierNonArray::IVec4 => 16,
            syntax::TypeSpecifierNonArray::UVec2 => 8,
            syntax::TypeSpecifierNonArray::UVec3 => 12,
            syntax::TypeSpecifierNonArray::UVec4 => 16,
            syntax::TypeSpecifierNonArray::Mat2 => 4 * 4,
            syntax::TypeSpecifierNonArray::Mat3 => 9 * 4,
            syntax::TypeSpecifierNonArray::Mat4 => 16 * 4,
            syntax::TypeSpecifierNonArray::Mat23 => 6 * 4,
            syntax::TypeSpecifierNonArray::Mat24 => 8 * 4,
            syntax::TypeSpecifierNonArray::Mat32 => 6 * 4,
            syntax::TypeSpecifierNonArray::Mat34 => 12 * 4,
            syntax::TypeSpecifierNonArray::Mat42 => 8 * 4,
            syntax::TypeSpecifierNonArray::Mat43 => 12 * 4,
            unexpected => panic!("Haven't implemented size map for type {unexpected:?}"),
        })
    }
}

fn match_block_field(block_field: &syntax::StructFieldSpecifier) -> VResult<BlockField> {
    let syntax::StructFieldSpecifier {
        qualifier: type_qualifier,
        ty: type_specifier,
        identifiers: syntax::NonEmpty(identifiers),
    } = block_field;

    let type_properties = type_qualifier
        .as_ref()
        .map(simplify_type_qualifier)
        .transpose()?;

    let type_specifier = simplify_type_specifier(type_specifier)?;

    let arrayed_identifier = match &identifiers[..] {
        [x] => x,
        other => {
            let msg = format!("Unexpected identifiers: {other:?}");
            return Err(Error::Local(msg));
        }
    };
    let syntax::ArrayedIdentifier { ident, array_spec } = arrayed_identifier;
    let dimensions = array_spec
        .as_ref()
        .map(|array_specifier| {
            let syntax::ArraySpecifier {
                dimensions: syntax::NonEmpty(dimensions),
            } = array_specifier;
            dimensions
                .iter()
                .map(|sizing| {
                    if let syntax::ArraySpecifierDimension::ExplicitlySized(expr_box) = sizing {
                        if let syntax::Expr::IntConst(value) = **expr_box {
                            Ok(Some(usize::try_from(value).unwrap()))
                        } else {
                            let msg = format!("Unexpected array dimension value: {:?}", **expr_box);
                            Err(Error::Local(msg))
                        }
                    } else {
                        Ok(None)
                    }
                })
                .collect::<VResult<Vec<Option<usize>>>>()
        })
        .transpose()?;

    Ok(BlockField {
        name: ident.to_string(),
        type_specifier,
        offset: type_properties.and_then(|p| p.offset),
        dimensions,
    })
}

#[derive(Debug)]
pub struct BlockDeclaration {
    struct_name: String,
    variable_name: Option<String>,
    pub push_constant: bool,
    pub storage: vk::DescriptorType,
    pub binding: Option<usize>,
    pub set: Option<usize>,
    pub fields: Vec<BlockField>,
}

impl DescriptorInfo for BlockDeclaration {
    fn storage(&self) -> vk::DescriptorType {
        self.storage
    }

    fn set_index(&self) -> usize {
        self.set.unwrap_or_else(|| {
            warn!("Assuming set=0 for block {}", self.struct_name);
            0 // TODO move this to parsing stage.
        })
    }

    fn binding(&self) -> VResult<usize> {
        self.binding.ok_or_else(|| {
            let msg = format!("Block '{}' does not specify a binding.", self.struct_name);
            Error::Local(msg)
        }) // TODO move to parsing stage?
    }

    fn name(&self) -> &str {
        self.variable_name.as_ref().unwrap_or(&self.struct_name)
    }
}

/// Alignment cannot be less than 4. Even booleans should be aligned to 4 bytes...
fn alignment(x: usize) -> usize {
    let exp = ((x as f32).log2().ceil() as u32).max(2);
    2usize.pow(exp)
}

impl BlockDeclaration {
    #[must_use]
    pub fn byte_size(&self) -> Option<usize> {
        let mut max_size = 0;
        for field in &self.fields {
            let byte_size = field.byte_size()?;
            let offset = field.offset?;

            max_size = max_size.max(offset + alignment(byte_size));
        }
        Some(max_size)
    }
}

fn match_block(block: &syntax::Block) -> VResult<BlockDeclaration> {
    let syntax::Block {
        qualifier: type_qualifier,
        name,
        fields,
        identifier,
    } = block;

    let identifier = identifier
        .as_ref()
        .map(|arrayed_identifier| {
            let syntax::ArrayedIdentifier { ident, array_spec } = arrayed_identifier;
            if array_spec.is_some() {
                let msg = format!("Unexpected array spec: {array_spec:?}");
                return Err(Error::Local(msg));
            }
            Ok(ident.to_string())
        })
        .transpose()?;

    let type_properties = simplify_type_qualifier(type_qualifier)?;

    let storage = match type_properties.storage {
        Some(syntax::StorageQualifier::Uniform) => vk::DescriptorType::UNIFORM_BUFFER,
        Some(syntax::StorageQualifier::Buffer) => vk::DescriptorType::STORAGE_BUFFER,
        other => {
            let msg = format!("Invalid storage qualifier for block: {other:?}");
            return Err(Error::Local(msg));
        }
    };

    let fields = fields
        .iter()
        .map(match_block_field)
        .collect::<VResult<_>>()?;

    Ok(BlockDeclaration {
        struct_name: name.to_string(),
        variable_name: identifier,
        push_constant: type_properties.push_constant,
        storage,
        binding: type_properties.binding,
        set: type_properties.set,
        fields,
    })
}

pub type LocalSize = (usize, usize, usize);
pub type ShaderIO = (LocalSize, Vec<VariableDeclaration>, Vec<BlockDeclaration>);

pub fn analyze_shader(path: &Path) -> VResult<ShaderIO> {
    let shader_code = fs::read_to_string(path).map_err(|err| {
        Error::Local(format!("File '{}' cannot be read: {err:?}", path.display()))
    })?;
    let syntax::TranslationUnit(syntax::NonEmpty(external_declarations)) =
        syntax::ShaderStage::parse(shader_code)?;

    let mut local_size = (1, 1, 1);
    let mut declarations = Vec::new();
    let mut blocks = Vec::new();

    for external_declaration in &external_declarations {
        match external_declaration {
            syntax::ExternalDeclaration::Declaration(declaration) => match declaration {
                // Global declarations include the local size of the shader.
                // This is relevant for the dispatch size.
                syntax::Declaration::Global(type_qualifier, global_names) => {
                    local_size = match_globals(type_qualifier, global_names)?;
                }
                // Init declarator lists define images accessed via samplers.
                syntax::Declaration::InitDeclaratorList(init_declarator_list) => {
                    match_init_declarator_list(init_declarator_list)?
                        .into_iter()
                        .for_each(|declaration| declarations.push(declaration));
                }
                syntax::Declaration::Block(block) => blocks.push(match_block(block)?),
                // Ignore the following.
                syntax::Declaration::Precision(..) | syntax::Declaration::FunctionPrototype(..) => {
                }
            },
            // Ignore the following.
            syntax::ExternalDeclaration::Preprocessor(..)
            | syntax::ExternalDeclaration::FunctionDefinition(..) => {}
        }
    }

    Ok((local_size, declarations, blocks))
}
