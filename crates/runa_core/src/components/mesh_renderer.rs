use runa_asset::{Handle, TextureAsset};
use runa_macros::Scriptable;
use std::sync::Arc;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<u32>,
    pub submeshes: Vec<SubMesh>,
    pub bounds: Aabb,
    /// Legacy texture slot kept until materials fully own texture binding in the renderer.
    pub texture: Option<std::sync::Arc<TextureAsset>>,
    pub primitive_hint: Option<BuiltinMeshPrimitive>,
}

#[derive(Clone, Copy, Debug)]
pub struct SubMesh {
    pub index_start: u32,
    pub index_count: u32,
    pub material_slot: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Default for Aabb {
    fn default() -> Self {
        Self {
            min: [0.0, 0.0, 0.0],
            max: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Clone, Debug)]
pub struct Material {
    pub base_color: [f32; 4],
    pub base_color_texture: Option<Handle<TextureAsset>>,
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: Option<Handle<TextureAsset>>,
    pub normal_texture: Option<Handle<TextureAsset>>,
    pub occlusion_texture: Option<Handle<TextureAsset>>,
    pub use_vertex_color: bool,
    pub emission: [f32; 3],
    pub emissive_texture: Option<Handle<TextureAsset>>,
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
// pub enum ColorSpace {
//     Srgb,
//     Linear,
// }

// #[derive(Clone, Debug)]
// pub struct Texture {
//     pub path: String,
//     pub color_space: ColorSpace,
// }

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic: 0.0,
            roughness: 1.0,
            metallic_roughness_texture: None,
            normal_texture: None,
            occlusion_texture: None,
            use_vertex_color: false,
            emission: [0.0, 0.0, 0.0],
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinMeshPrimitive {
    Cube,
    Quad,
    Plane,
    Pyramid,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex3D>, indices: Vec<u32>) -> Self {
        let bounds = Self::calculate_bounds(&vertices);
        let submeshes = vec![SubMesh {
            index_start: 0,
            index_count: indices.len() as u32,
            material_slot: 0,
        }];
        Self {
            vertices,
            indices,
            submeshes,
            bounds,
            texture: None,
            primitive_hint: None,
        }
    }

    pub fn calculate_bounds(vertices: &[Vertex3D]) -> Aabb {
        if vertices.is_empty() {
            return Aabb::default();
        }
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for vertex in vertices {
            for axis in 0..3 {
                min[axis] = min[axis].min(vertex.position[axis]);
                max[axis] = max[axis].max(vertex.position[axis]);
            }
        }
        Aabb { min, max }
    }

    pub fn cube(size: f32) -> Self {
        let h = size * 0.5;

        // Cube vertices (6 faces x 4 vertices = 24 vertices)
        let vertices = vec![
            // Front face (z = h)
            Vertex3D {
                position: [-h, -h, h],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, -h, h],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, h],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, h, h],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // Back face (z = -h)
            Vertex3D {
                position: [-h, -h, -h],
                normal: [0.0, 0.0, -1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, h, -h],
                normal: [0.0, 0.0, -1.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, -h],
                normal: [0.0, 0.0, -1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, -h, -h],
                normal: [0.0, 0.0, -1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // Top face (y = h)
            Vertex3D {
                position: [-h, h, -h],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, h, h],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, h],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, -h],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // Bottom face (y = -h)
            Vertex3D {
                position: [-h, -h, -h],
                normal: [0.0, -1.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, -h, -h],
                normal: [0.0, -1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, -h, h],
                normal: [0.0, -1.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, -h, h],
                normal: [0.0, -1.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // Right face (x = h)
            Vertex3D {
                position: [h, -h, -h],
                normal: [1.0, 0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, -h],
                normal: [1.0, 0.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, h, h],
                normal: [1.0, 0.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [h, -h, h],
                normal: [1.0, 0.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            // Left face (x = -h)
            Vertex3D {
                position: [-h, -h, -h],
                normal: [-1.0, 0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, -h, h],
                normal: [-1.0, 0.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, h, h],
                normal: [-1.0, 0.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-h, h, -h],
                normal: [-1.0, 0.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];

        // Indices for 12 triangles (2 per face x 6 faces)
        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0, // Back
            4, 5, 6, 6, 7, 4, // Top
            8, 9, 10, 10, 11, 8, // Bottom
            12, 13, 14, 14, 15, 12, // Right
            16, 17, 18, 18, 19, 16, // Left
            20, 21, 22, 22, 23, 20,
        ];

        let mut mesh = Self::new(vertices, indices);
        mesh.primitive_hint = Some(BuiltinMeshPrimitive::Cube);
        mesh
    }

    pub fn quad(width: f32, height: f32) -> Self {
        let hw = width * 0.5;
        let hh = height * 0.5;
        let vertices = vec![
            Vertex3D {
                position: [-hw, -hh, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, -hh, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, hh, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, hh, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        let mut mesh = Self::new(vertices, indices);
        mesh.primitive_hint = Some(BuiltinMeshPrimitive::Quad);
        mesh
    }

    pub fn plane(width: f32, depth: f32) -> Self {
        let hw = width * 0.5;
        let hd = depth * 0.5;
        let vertices = vec![
            Vertex3D {
                position: [-hw, 0.0, -hd],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, 0.0, -hd],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, 0.0, hd],
                normal: [0.0, 1.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, 0.0, hd],
                normal: [0.0, 1.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        let mut mesh = Self::new(vertices, indices);
        mesh.primitive_hint = Some(BuiltinMeshPrimitive::Plane);
        mesh
    }

    pub fn pyramid(width: f32, height: f32, depth: f32) -> Self {
        let hw = width * 0.5;
        let hd = depth * 0.5;
        let apex = [0.0, height * 0.5, 0.0];
        let base_y = -height * 0.5;
        let vertices = vec![
            Vertex3D {
                position: [-hw, base_y, -hd],
                normal: [0.0, -1.0, 0.0],
                uv: [0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, -hd],
                normal: [0.0, -1.0, 0.0],
                uv: [1.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, hd],
                normal: [0.0, -1.0, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, base_y, hd],
                normal: [0.0, -1.0, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: apex,
                normal: [0.0, 0.707, -0.707],
                uv: [0.5, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, base_y, -hd],
                normal: [0.0, 0.707, -0.707],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, -hd],
                normal: [0.0, 0.707, -0.707],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: apex,
                normal: [0.707, 0.707, 0.0],
                uv: [0.5, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, -hd],
                normal: [0.707, 0.707, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, hd],
                normal: [0.707, 0.707, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: apex,
                normal: [0.0, 0.707, 0.707],
                uv: [0.5, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [hw, base_y, hd],
                normal: [0.0, 0.707, 0.707],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, base_y, hd],
                normal: [0.0, 0.707, 0.707],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: apex,
                normal: [-0.707, 0.707, 0.0],
                uv: [0.5, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, base_y, hd],
                normal: [-0.707, 0.707, 0.0],
                uv: [0.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
            Vertex3D {
                position: [-hw, base_y, -hd],
                normal: [-0.707, 0.707, 0.0],
                uv: [1.0, 1.0],
                color: [1.0, 1.0, 1.0, 1.0],
            },
        ];
        let indices = vec![0, 1, 2, 2, 3, 0, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut mesh = Self::new(vertices, indices);
        mesh.primitive_hint = Some(BuiltinMeshPrimitive::Pyramid);
        mesh
    }
}

#[derive(Clone, Scriptable)]
pub struct MeshRenderer {
    #[script(skip)]
    pub mesh: Option<Handle<Mesh>>,
    pub mesh_path: Option<String>,
    #[script(skip)]
    pub materials: Vec<Handle<Material>>,
    pub visible: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    /// Legacy tint kept for compatibility with existing project serialization/editor code.
    pub color: [f32; 4],
}

impl MeshRenderer {
    pub fn new(mesh: Mesh) -> Self {
        Self {
            mesh: Option::from(Handle::from(Arc::new(mesh))),
            mesh_path: Some("".to_string()),
            materials: vec![Handle::from(Arc::new(Material::default()))],
            visible: true,
            cast_shadows: true,
            receive_shadows: true,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn get_mesh_handle(&self) -> Handle<Mesh> {
        self.mesh.clone().unwrap()
    }

    pub fn set_mesh(&mut self, mesh: Option<Handle<Mesh>>, mesh_path: Option<String>) {
        self.mesh = mesh;
        self.mesh_path = mesh_path;
    }

    pub fn material(&self, slot: usize) -> Material {
        self.materials
            .get(slot)
            .map(|material| (*material.inner).clone())
            .unwrap_or_default()
    }

    pub fn set_material(&mut self, slot: usize, material: Material) {
        if self.materials.len() <= slot {
            self.materials
                .resize_with(slot + 1, || Handle::from(Arc::new(Material::default())));
        }
        self.materials[slot] = Handle::from(Arc::new(material));
    }

    pub fn material_for_rendering(&self) -> Material {
        let mut material = self.material(0);
        material.base_color = self.color;
        material
    }
}
