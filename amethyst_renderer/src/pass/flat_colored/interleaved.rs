//! Simple flat forward drawing pass.

use std::marker::PhantomData;

use derivative::Derivative;
use gfx::pso::buffer::ElemStride;
use gfx_core::state::{Blend, ColorMask};
use glsl_layout::Uniform;

use amethyst_assets::AssetStorage;
use amethyst_core::{
    ecs::prelude::{Join, Read, ReadExpect, ReadStorage},
    transform::GlobalTransform,
};
use amethyst_error::Error;

use crate::{
    cam::{ActiveCamera, Camera},
    hidden::{Hidden, HiddenPropagate},
    mesh::{Mesh, MeshHandle},
    mtl::{Material, MaterialDefaults},
    pass::util::{draw_mesh, get_camera, VertexArgs},
    pipe::{
        pass::{Pass, PassData},
        DepthMode, Effect, NewEffect,
    },
    tex::Texture,
    types::{Encoder, Factory},
    vertex::{Color, Position, Query},
    visibility::Visibility,
    Rgba,
};

use super::*;

/// Draw mesh without lighting
///
/// See the [crate level documentation](index.html) for information about interleaved and separate
/// passes.
///
/// # Type Parameters
///
/// * `V`: `VertexFormat`
#[derive(Derivative, Clone, Debug, PartialEq)]
#[derivative(Default(bound = "V: Query<(Position, Color)>, Self: Pass"))]
pub struct DrawFlatColored<V> {
    _pd: PhantomData<V>,
    transparency: Option<(ColorMask, Blend, Option<DepthMode>)>,
}

impl<V> DrawFlatColored<V>
where
    V: Query<(Position, Color)>,
    Self: Pass,
{
    /// Create instance of `DrawFlatColored` pass
    pub fn new() -> Self {
        Default::default()
    }

    /// Enable transparency
    pub fn with_transparency(
        mut self,
        mask: ColorMask,
        blend: Blend,
        depth: Option<DepthMode>,
    ) -> Self {
        self.transparency = Some((mask, blend, depth));
        self
    }
}

impl<'a, V> PassData<'a> for DrawFlatColored<V>
where
    V: Query<(Position, Color)>,
{
    type Data = (
        Read<'a, ActiveCamera>,
        ReadStorage<'a, Camera>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AssetStorage<Texture>>,
        ReadExpect<'a, MaterialDefaults>,
        Option<Read<'a, Visibility>>,
        ReadStorage<'a, Hidden>,
        ReadStorage<'a, HiddenPropagate>,
        ReadStorage<'a, MeshHandle>,
        ReadStorage<'a, Material>,
        ReadStorage<'a, GlobalTransform>,
        ReadStorage<'a, Rgba>,
    );
}

impl<V> Pass for DrawFlatColored<V>
where
    V: Query<(Position, Color)>,
{
    fn compile(&mut self, effect: NewEffect<'_>) -> Result<Effect, Error> {
        use std::mem;
        let mut builder = effect.simple(VERT_SRC, FRAG_SRC);
        builder
            .with_raw_constant_buffer(
                "VertexArgs",
                mem::size_of::<<VertexArgs as Uniform>::Std140>(),
                1,
            )
            .with_raw_vertex_buffer(V::QUERIED_ATTRIBUTES, V::size() as ElemStride, 0);
        match self.transparency {
            Some((mask, blend, depth)) => builder.with_blended_output("color", mask, blend, depth),
            None => builder.with_output("color", Some(DepthMode::LessEqualWrite)),
        };
        builder.build()
    }

    fn apply<'a, 'b: 'a>(
        &'a mut self,
        encoder: &mut Encoder,
        effect: &mut Effect,
        _factory: Factory,
        (
            active,
            camera,
            mesh_storage,
            tex_storage,
            material_defaults,
            visibility,
            hidden,
            hidden_prop,
            mesh,
            material,
            global,
            rgba,
        ): <Self as PassData<'a>>::Data,
    ) {
        let camera = get_camera(active, &camera, &global);

        match visibility {
            None => {
                for (mesh, material, global, rgba, _, _) in (
                    &mesh,
                    &material,
                    &global,
                    rgba.maybe(),
                    !&hidden,
                    !&hidden_prop,
                )
                    .join()
                {
                    draw_mesh(
                        encoder,
                        effect,
                        false,
                        mesh_storage.get(mesh),
                        None,
                        &tex_storage,
                        Some(material),
                        &material_defaults,
                        rgba,
                        camera,
                        Some(global),
                        &[V::QUERIED_ATTRIBUTES],
                        &[],
                    );
                }
            }
            Some(ref visibility) => {
                for (mesh, material, global, rgba, _) in (
                    &mesh,
                    &material,
                    &global,
                    rgba.maybe(),
                    &visibility.visible_unordered,
                )
                    .join()
                {
                    draw_mesh(
                        encoder,
                        effect,
                        false,
                        mesh_storage.get(mesh),
                        None,
                        &tex_storage,
                        Some(material),
                        &material_defaults,
                        rgba,
                        camera,
                        Some(global),
                        &[V::QUERIED_ATTRIBUTES],
                        &[],
                    );
                }

                for entity in &visibility.visible_ordered {
                    if let Some(mesh) = mesh.get(*entity) {
                        draw_mesh(
                            encoder,
                            effect,
                            false,
                            mesh_storage.get(mesh),
                            None,
                            &tex_storage,
                            material.get(*entity),
                            &material_defaults,
                            rgba.get(*entity),
                            camera,
                            global.get(*entity),
                            &[V::QUERIED_ATTRIBUTES],
                            &[],
                        );
                    }
                }
            }
        }
    }
}
