use std::marker::PhantomData;

use bevy::{
    core_pipeline::Transparent2d,
    prelude::{
        Assets, Commands, Component, CoreStage, Entity, Plugin, Query, RemovedComponents, Shader,
        With,
    },
    render::{
        mesh::MeshVertexAttribute,
        render_phase::AddRenderCommand,
        render_resource::{DynamicUniformVec, SpecializedRenderPipelines, VertexFormat},
        RenderApp, RenderStage,
    },
};

use crate::tiles::TilePos2d;

use self::{
    chunk::{RenderChunk2dStorage, TilemapUniformData},
    draw::DrawTilemap,
    pipeline::{
        TilemapPipeline, HEX_COLUMN_EVEN_SHADER_HANDLE, HEX_COLUMN_ODD_SHADER_HANDLE,
        HEX_COLUMN_SHADER_HANDLE, HEX_ROW_EVEN_SHADER_HANDLE, HEX_ROW_ODD_SHADER_HANDLE,
        HEX_ROW_SHADER_HANDLE, ISO_DIAMOND_SHADER_HANDLE, ISO_STAGGERED_SHADER_HANDLE,
        SQUARE_SHADER_HANDLE,
    },
    prepare::MeshUniform,
    queue::ImageBindGroups,
};

mod chunk;
mod draw;
mod extract;
mod include_shader;
mod pipeline;
mod prepare;
mod queue;

pub struct Tilemap2dRenderingPlugin;

impl Plugin for Tilemap2dRenderingPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system_to_stage(CoreStage::First, clear_removed);
        app.add_system_to_stage(CoreStage::PostUpdate, removal_helper);

        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();

        let tilemap_shader = include_str!("shaders/tilemap-atlas.wgsl");

        let square_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/square.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(SQUARE_SHADER_HANDLE, square_shader);

        let iso_diamond_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/diamond_iso.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(ISO_DIAMOND_SHADER_HANDLE, iso_diamond_shader);

        let iso_staggered_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/staggered_iso.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(ISO_STAGGERED_SHADER_HANDLE, iso_staggered_shader);

        let hex_column_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/column_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_COLUMN_SHADER_HANDLE, hex_column_shader);

        let hex_column_odd_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/column_odd_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_COLUMN_ODD_SHADER_HANDLE, hex_column_odd_shader);

        let hex_column_even_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/column_even_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_COLUMN_EVEN_SHADER_HANDLE, hex_column_even_shader);

        let hex_row_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/row_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_ROW_SHADER_HANDLE, hex_row_shader);

        let hex_row_odd_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/row_odd_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_ROW_ODD_SHADER_HANDLE, hex_row_odd_shader);

        let hex_row_even_shader = Shader::from_wgsl(include_shader::include_shader(
            vec![include_str!("shaders/row_even_hex.wgsl")],
            tilemap_shader,
        ));
        shaders.set_untracked(HEX_ROW_EVEN_SHADER_HANDLE, hex_row_even_shader);

        // app.add_plugin(UniformComponentPlugin::<MeshUniform>::default());
        // app.add_plugin(UniformComponentPlugin::<TilemapUniformData>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.insert_resource(RenderChunk2dStorage::default());
        render_app
            .add_system_to_stage(RenderStage::Extract, extract::extract)
            .add_system_to_stage(RenderStage::Extract, extract::extract_removal);
        render_app
            .add_system_to_stage(RenderStage::Prepare, prepare::prepare)
            .add_system_to_stage(RenderStage::Prepare, prepare::prepare_removal)
            .add_system_to_stage(RenderStage::Queue, queue::queue_meshes)
            .add_system_to_stage(RenderStage::Queue, queue::queue_transform_bind_group)
            .add_system_to_stage(RenderStage::Queue, queue::queue_tilemap_bind_group)
            .init_resource::<TilemapPipeline>()
            .init_resource::<ImageBindGroups>()
            .init_resource::<SpecializedRenderPipelines<TilemapPipeline>>()
            .init_resource::<DynamicUniformVec<MeshUniform>>()
            .init_resource::<DynamicUniformVec<TilemapUniformData>>();

        render_app.add_render_command::<Transparent2d, DrawTilemap>();
    }
}

/// Stores the index of a uniform inside of [`ComponentUniforms`].
#[derive(Component)]
pub struct DynamicUniformIndex<C: Component> {
    index: u32,
    marker: PhantomData<C>,
}

impl<C: Component> DynamicUniformIndex<C> {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub const ATTRIBUTE_POSITION: MeshVertexAttribute =
    MeshVertexAttribute::new("Position", 229221259, VertexFormat::Float32x4);
pub const ATTRIBUTE_TEXTURE: MeshVertexAttribute =
    MeshVertexAttribute::new("Texture", 222922753, VertexFormat::Float32x4);
pub const ATTRIBUTE_COLOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Color", 231497124, VertexFormat::Float32x4);

#[derive(Component)]
pub struct RemovedTileEntity(pub Entity);

fn removal_helper(mut commands: Commands, removed_query: RemovedComponents<TilePos2d>) {
    for entity in removed_query.iter() {
        commands.spawn().insert(RemovedTileEntity(entity));
    }
}

fn clear_removed(mut commands: Commands, removed_query: Query<Entity, With<RemovedTileEntity>>) {
    for entity in removed_query.iter() {
        commands.entity(entity).despawn();
    }
}
