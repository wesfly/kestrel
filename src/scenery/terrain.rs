use crate::{EARTH_RADIUS, TOKIO_RUNTIME, absolute_position};
use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::GREEN,
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    math::{DQuat, DVec3},
    mesh::{Indices, PrimitiveTopology},
    pbr::ExtendedMaterial,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures_lite::future},
};
use big_space::prelude::*;
use dashmap::DashMap;
pub use material::TerrainMaterial;
use reqwest::Client;
use serde::Deserialize;
use std::fs::File;
use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{OnceCell, Semaphore};

mod material;

const SIZE: f32 = 2.0;
const SUBDIV_PER_TILE: u32 = 32;

/// How many chunk-widths away from a node's centre the reference point
/// needs to be before a *currently coarse* node splits into finer detail.
const LOD_SPLIT_FACTOR: f32 = 2.2;

/// How many chunk-widths away a *currently split* node needs to retreat to
/// before merging back into its coarser parent. This is larger than
/// `LOD_SPLIT_FACTOR` to avoid flickering at points where tiles would get
/// spawned and then despanwned a few frames later.
const LOD_MERGE_FACTOR: f32 = 2.8;

/// Deepest zoom the Mapterhorn heightmap tiles actually have data for.
const MAX_HEIGHTMAP_ZOOM: u8 = 15;

/// How far skirts drop below the surface, as a fraction of the chunk's own
/// size.
const SKIRT_DEPTH_FACTOR: f32 = 0.15;

/// How far (in world units) the reference point has to move since the last
/// update before the quadtree is re-evaluated.
const MIN_UPDATE_MOVEMENT: f32 = 250.0;

/// Resource holding the TileCache
#[derive(Resource, Clone)]
pub struct TerrainCacheResource {
    pub cache: TileCache,
}
impl Default for TerrainCacheResource {
    fn default() -> Self {
        return Self {
            cache: Arc::new(DashMap::new()),
        };
    }
}

/// Persistent HTTP client for heightmap tile fetches.
#[derive(Resource, Clone)]
pub struct HttpClient(pub Client);

/// Persistent semaphore capping concurrent tile downloads.
#[derive(Resource, Clone)]
pub struct TileSemaphore(pub Arc<Semaphore>);

/// Tracks which quadtree leaf chunks are currently spawned (or in flight)
#[derive(Resource, Default)]
pub struct TerrainChunkRegistry {
    live: HashMap<NodeKey, Entity>,
    pending: HashSet<NodeKey>,
    /// To be despawned
    retiring: HashMap<NodeKey, Entity>,
    /// To be split
    split: HashSet<NodeKey>,
}

/// Gates how often `update_terrain_for_aircraft` actually re-walks the
/// quadtree.
#[derive(Resource, Default)]
pub struct TerrainUpdateTracker {
    last_reference_pos: Option<Vec3>,
}

#[derive(Resource, Deserialize, Clone, Copy, Debug)]
pub struct TerrainSettings {
    /// Continously updated for `update_terrain_for_aircraft`
    pub coord: Coord,
    max_render_distance: f32,
    /// Maximum quadtree depth
    pub level_of_detail: u8,
}

impl Default for TerrainSettings {
    fn default() -> Self {
        TerrainSettings {
            coord: Coord {
                lat: 0.0,
                long: 0.0,
            },
            level_of_detail: 12,
            max_render_distance: 100_000.0,
        }
    }
}

#[derive(Component)]
struct TerrainFaces;

#[derive(Component)]
pub struct SpawnTerrain {
    task: Task<Option<(Mesh, Collider)>>,
    key: NodeKey,
    cell_coord: CellCoord,
    cell_offset: Vec3,
}

#[derive(Copy, Clone, PartialEq, Deserialize, Debug, Default)]
pub struct Coord {
    pub lat: f32,
    pub long: f32,
}

impl Coord {
    pub fn from(lat: f32, long: f32) -> Self {
        Self { lat, long }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CubeFace {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

impl CubeFace {
    pub const ALL: [CubeFace; 6] = [
        CubeFace::PosX,
        CubeFace::NegX,
        CubeFace::PosY,
        CubeFace::NegY,
        CubeFace::PosZ,
        CubeFace::NegZ,
    ];

    fn normal(self) -> Dir3 {
        match self {
            CubeFace::PosX => Dir3::X,
            CubeFace::NegX => Dir3::NEG_X,
            CubeFace::PosY => Dir3::Y,
            CubeFace::NegY => Dir3::NEG_Y,
            CubeFace::PosZ => Dir3::Z,
            CubeFace::NegZ => Dir3::NEG_Z,
        }
    }
}

/// Uniquely identifies one quadtree node (leaf or not) across all six faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey {
    face: CubeFace,
    level: u8,
    x: u32,
    y: u32,
}

impl NodeKey {
    fn parent(self) -> Option<NodeKey> {
        if self.level == 0 {
            None
        } else {
            Some(NodeKey {
                face: self.face,
                level: self.level - 1,
                x: self.x / 2,
                y: self.y / 2,
            })
        }
    }

    fn children(self) -> [NodeKey; 4] {
        let level = self.level + 1;
        let x = self.x * 2;
        let y = self.y * 2;
        [
            NodeKey {
                face: self.face,
                level,
                x,
                y,
            },
            NodeKey {
                face: self.face,
                level,
                x: x + 1,
                y,
            },
            NodeKey {
                face: self.face,
                level,
                x,
                y: y + 1,
            },
            NodeKey {
                face: self.face,
                level,
                x: x + 1,
                y: y + 1,
            },
        ]
    }
}

#[derive(Clone, Copy, Debug)]
struct QuadtreeNode {
    level: u8,
    x: u32,
    y: u32,
}

impl QuadtreeNode {
    fn root() -> Self {
        Self {
            level: 0,
            x: 0,
            y: 0,
        }
    }

    fn children(self) -> [QuadtreeNode; 4] {
        let level = self.level + 1;
        let x = self.x * 2;
        let y = self.y * 2;
        [
            QuadtreeNode { level, x, y },
            QuadtreeNode { level, x: x + 1, y },
            QuadtreeNode { level, x, y: y + 1 },
            QuadtreeNode {
                level,
                x: x + 1,
                y: y + 1,
            },
        ]
    }
}

/// World-space (pre-sphere-warp) centre and edge length of a quadtree node's
/// square footprint on the given cube face.
fn node_transform(node: QuadtreeNode, normal: Dir3) -> (Vec3, f32) {
    let tiles_per_axis = 1u32 << (node.level as u32);
    let chunk_size = SIZE / tiles_per_axis as f32;
    let centre_offset = (tiles_per_axis as f32 - 1.0) * 0.5;

    let a = (node.x as f32 - centre_offset) * chunk_size;
    let b = (node.y as f32 - centre_offset) * chunk_size;

    let mut translation_per_chunk = Vec3::ZERO;
    if normal == Dir3::NEG_X || normal == Dir3::X {
        translation_per_chunk.y = a;
        translation_per_chunk.z = b;
    }
    if normal == Dir3::NEG_Y || normal == Dir3::Y {
        translation_per_chunk.x = a;
        translation_per_chunk.z = b;
    }
    if normal == Dir3::NEG_Z || normal == Dir3::Z {
        translation_per_chunk.x = a;
        translation_per_chunk.y = b;
    }

    let chunk_translation = Vec3 {
        x: normal.x,
        y: normal.y,
        z: normal.z,
    } + translation_per_chunk;

    (chunk_translation, chunk_size)
}

/// World-space version of `coord_to_pos`. Is equal to `coord_to_pos * EARTH_RADIUS`.
pub fn coord_to_world_pos(coord: Coord) -> DVec3 {
    coord_to_pos(coord).as_dvec3() * EARTH_RADIUS as f64
}

/// Walks the quadtree for one face, recording every node that should
/// currently be a visible leaf chunk into `desired`, and every node that's
/// currently split into `newly_split`.
#[allow(clippy::too_many_arguments)]
fn collect_desired_nodes(
    face: CubeFace,
    node: QuadtreeNode,
    terrain: &TerrainSettings,
    reference_pos: Vec3,
    previously_split: &HashSet<NodeKey>,
    desired: &mut HashSet<NodeKey>,
    newly_split: &mut HashSet<NodeKey>,
) {
    let (chunk_translation, chunk_size) = node_transform(node, face.normal());
    let projected_chunk_center = to_sphere_pos(chunk_translation.as_dvec3()).as_vec3();

    let distance = projected_chunk_center.distance(reference_pos);
    let node_world_size = chunk_size * EARTH_RADIUS;

    if distance - node_world_size * 0.5 > terrain.max_render_distance {
        return;
    }

    let key = NodeKey {
        face,
        level: node.level,
        x: node.x,
        y: node.y,
    };

    let split_threshold = if previously_split.contains(&key) {
        LOD_MERGE_FACTOR
    } else {
        LOD_SPLIT_FACTOR
    };

    let should_split =
        node.level < terrain.level_of_detail && distance < node_world_size * split_threshold;

    if should_split {
        newly_split.insert(key);
        for child in node.children() {
            collect_desired_nodes(
                face,
                child,
                terrain,
                reference_pos,
                previously_split,
                desired,
                newly_split,
            );
        }
    } else {
        desired.insert(key);
    }
}

/// Re-evaluates the quadtree around `reference_pos` across all six faces and
/// spawns/despawns chunks to match.
pub fn update_terrain(
    commands: &mut Commands,
    grid: &Grid,
    registry: &mut TerrainChunkRegistry,
    client: &Client,
    semaphore: Arc<Semaphore>,
    cache: TileCache,
    terrain: TerrainSettings,
    reference_pos: Vec3,
) {
    let mut desired = HashSet::new();
    let mut newly_split = HashSet::new();

    for face in CubeFace::ALL {
        collect_desired_nodes(
            face,
            QuadtreeNode::root(),
            &terrain,
            reference_pos,
            &registry.split,
            &mut desired,
            &mut newly_split,
        );
    }
    registry.split = newly_split;

    let TerrainChunkRegistry {
        live,
        pending,
        retiring,
        ..
    } = registry;

    live.retain(|key, entity| {
        if desired.contains(key) {
            true
        } else {
            retiring.insert(*key, *entity);
            false
        }
    });
    // Drop everything but the still desired keys
    pending.retain(|key| desired.contains(key));

    for key in &desired {
        if live.contains_key(key) || pending.contains(key) {
            continue;
        }
        if let Some(entity) = retiring.remove(key) {
            // Was about to be retired but became desired again before we
            // got around to despawning it (e.g. the reference point moved
            // back) - just reclaim it instead of rebuilding from scratch.
            live.insert(*key, entity);
            continue;
        }
        spawn_leaf(commands, grid, *key, client, &semaphore, &cache);
        pending.insert(*key);
    }

    // Only despawn a retiring chunk once whatever's actually replacing it
    // is live.
    retiring.retain(|key, entity| {
        let merging_into_parent = key.parent().is_some_and(|parent| desired.contains(&parent));
        let splitting_into_children = key.children().iter().any(|c| desired.contains(c));

        let ready = if merging_into_parent {
            if let Some(parent_key) = &key.parent() {
                warn!("{key:?} has no parent");
                live.contains_key(&parent_key)
            } else {
                false
            }
        } else if splitting_into_children {
            key.children().iter().all(|c| live.contains_key(c))
        } else {
            true
        };

        if ready {
            commands.entity(*entity).try_despawn();
            false
        } else {
            true
        }
    });
}

pub fn update_terrain_for_aircraft(
    commands: &mut Commands,
    grid: &Grid,
    registry: &mut TerrainChunkRegistry,
    tracker: &mut TerrainUpdateTracker,
    client: &Client,
    semaphore: Arc<Semaphore>,
    cache: TileCache,
    mut terrain: TerrainSettings,
    reference_pos: Vec3,
    override_update: bool,
) -> TerrainSettings {
    if let Some(last) = tracker.last_reference_pos {
        if last.distance(reference_pos) < MIN_UPDATE_MOVEMENT && !override_update {
            return terrain;
        }
    }
    tracker.last_reference_pos = Some(reference_pos);
    terrain.coord = pos_to_coord(reference_pos);

    update_terrain(
        commands,
        grid,
        registry,
        client,
        semaphore,
        cache,
        terrain,
        reference_pos,
    );

    terrain
}

fn spawn_leaf(
    commands: &mut Commands,
    grid: &Grid,
    key: NodeKey,
    client: &Client,
    semaphore: &Arc<Semaphore>,
    cache: &TileCache,
) {
    let node = QuadtreeNode {
        level: key.level,
        x: key.x,
        y: key.y,
    };
    let (chunk_translation, chunk_size) = node_transform(node, key.face.normal());
    let projected_chunk_center = to_sphere_pos(chunk_translation.as_dvec3());

    let thread_pool = AsyncComputeTaskPool::get();
    let client_clone = client.clone();
    let semaphore_clone = Arc::clone(semaphore);

    let tokio_handle = TOKIO_RUNTIME.spawn(build_mesh(
        key.face.normal(),
        chunk_translation,
        chunk_size,
        key.level,
        client_clone,
        semaphore_clone,
        Arc::clone(cache),
    ));

    let task = thread_pool.spawn(async move { tokio_handle.await.unwrap_or_default() });
    let (cell_coord, cell_offset) = grid.translation_to_grid(projected_chunk_center);
    commands.spawn(SpawnTerrain {
        task,
        key,
        cell_coord,
        cell_offset,
    });
}

/// Returns a normalized Vec3; Use `coord_to_world_pos` for absolute position.
pub fn coord_to_pos(target_coord: Coord) -> Vec3 {
    let lat_rad = target_coord.lat.to_radians();
    let long_rad = target_coord.long.to_radians();

    let y = lat_rad.sin();
    let x = lat_rad.cos() * long_rad.sin();
    let z = lat_rad.cos() * long_rad.cos();
    Vec3::new(x, y, z).normalize()
}

/// Almost the same thing as normalizing the position, but it also evenly spaces the vertices on the sphere.
fn to_sphere_pos(pos: DVec3) -> DVec3 {
    let x2 = pos.x * pos.x;
    let y2 = pos.y * pos.y;
    let z2 = pos.z * pos.z;

    let x = pos.x * (1.0 - (y2 + z2) / 2.0 + (y2 * z2 / 3.0)).sqrt();
    let y = pos.y * (1.0 - (z2 + x2) / 2.0 + (z2 * x2 / 3.0)).sqrt();
    let z = pos.z * (1.0 - (x2 + y2) / 2.0 + (x2 * y2 / 3.0)).sqrt();

    DVec3::new(x, y, z) * EARTH_RADIUS as f64
}

/// World-space position of the height-displaced surface at `coord`.
fn world_surface_pos(coord: Coord, zoom: u8, cache: &TileCache) -> DVec3 {
    let height = get_height_at_coord(coord, zoom, cache);
    let factor = 1.0 + height as f64 / EARTH_RADIUS as f64;
    coord_to_world_pos(coord) * factor
}

/// Sampling distance from original vertex, in degrees
const NORMAL_SAMPLE_STEP_DEGREES: f32 = 0.0002;

/// The four coordinates `sample_analytic_normal` reads around `coord` to
/// estimate the surface slope.
fn normal_sample_coords(coord: Coord) -> [Coord; 4] {
    let lat_step = NORMAL_SAMPLE_STEP_DEGREES;
    // NOTE: This feels janky
    let long_step = NORMAL_SAMPLE_STEP_DEGREES / coord.lat.to_radians().cos().max(0.1);

    [
        Coord {
            lat: (coord.lat + lat_step).clamp(-85.0, 85.0),
            long: coord.long,
        },
        Coord {
            lat: (coord.lat - lat_step).clamp(-85.0, 85.0),
            long: coord.long,
        },
        Coord {
            lat: coord.lat,
            long: coord.long + long_step,
        },
        Coord {
            lat: coord.lat,
            long: coord.long - long_step,
        },
    ]
}

/// Computes a normal from the heightmap's local slope at `coord`.
async fn sample_analytic_normal(coord: Coord, zoom: u8, cache: &TileCache) -> Vec3 {
    let [north, south, east, west] = normal_sample_coords(coord);

    let pos_north = world_surface_pos(north, zoom, cache);
    let pos_south = world_surface_pos(south, zoom, cache);
    let pos_east = world_surface_pos(east, zoom, cache);
    let pos_west = world_surface_pos(west, zoom, cache);

    let tangent_north = pos_north - pos_south;
    let tangent_east = pos_east - pos_west;

    let normal_d = tangent_east.cross(tangent_north);
    let normal = if normal_d.length_squared() > 1e-12 {
        normal_d.normalize().as_vec3()
    } else {
        // Fallback
        coord_to_pos(coord)
    };

    // Normals should never point down
    let outward = coord_to_pos(coord);
    if normal.dot(outward) < 0.0 {
        -normal
    } else {
        normal
    }
}

/// One tile's fetch state, shared by every chunk-build that needs it.
pub enum TileState {
    Loaded(Arc<image::RgbImage>),
    /// The tile server has no coverage here - happens over water
    NoData,
    /// Fetching failed, system will retry fetching in `TILE_RETRY_COOLDOWN`.
    Failed(Instant),
}

pub type TileSlot = Arc<OnceCell<TileState>>;
pub type TileCache = Arc<DashMap<(u8, u32, u32), TileSlot>>;

/// How long a failed tile fetch is remembered before the next chunk that
/// needs it gets a fresh attempt.
const TILE_RETRY_COOLDOWN: Duration = Duration::from_secs(15);

fn coord_to_tile(coord: Coord, n: f32) -> (u32, u32) {
    let x = n * ((coord.long + 180.0) / 360.0);

    let lat_rad = coord
        .lat
        .to_radians()
        .clamp(-85.05112_f32.to_radians(), 85.05112_f32.to_radians());
    let y = (1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / std::f32::consts::PI) / 2.0 * n;

    (x.floor() as u32, y.floor() as u32)
}

/// Ensures every tile in `required_tiles` is either already cached or gets fetched.
async fn ensure_tiles_loaded(
    client: &Client,
    semaphore: Arc<Semaphore>,
    cache: TileCache,
    required_tiles: Vec<(u8, u32, u32)>,
) {
    let mut waiters = Vec::with_capacity(required_tiles.len());

    for key @ (zoom, x, y) in required_tiles {
        // Drop failed failed tiles from the cache to try fetching them again
        if let Some(slot) = cache.get(&key) {
            if let Some(TileState::Failed(failed_at)) = slot.get() {
                if failed_at.elapsed() > TILE_RETRY_COOLDOWN {
                    drop(slot);
                    cache.remove(&key);
                }
            }
        }

        let slot = cache
            .entry(key)
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone();

        let client = client.clone();
        let semaphore = Arc::clone(&semaphore);

        waiters.push(async move {
            slot.get_or_init(|| fetch_and_decode_tile(client, semaphore, zoom, x, y))
                .await;
        });
    }

    futures::future::join_all(waiters).await;
}

/// Downloads (if not already on disk) and decodes a single heightmap tile.
async fn fetch_and_decode_tile(
    client: Client,
    semaphore: Arc<Semaphore>,
    zoom: u8,
    x: u32,
    y: u32,
) -> TileState {
    let path = format!(".user/cache/elevation/{}_{}_{}.webp", zoom, x, y);

    match get_tile(&client, semaphore, &TerrariumCoords { z: zoom, x, y }).await {
        Ok(FetchOutcome::NoData) => return TileState::NoData,
        Err(e) => {
            warn!("Failed to fetch heightmap tile {zoom}/{x}/{y}: {e}");
            return TileState::Failed(Instant::now());
        }
        Ok(FetchOutcome::Downloaded) => {}
    }

    let decode_result = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        image::load_from_memory_with_format(&bytes, image::ImageFormat::WebP)
            .map(|img| Arc::new(img.to_rgb8()))
            .map_err(|e| e.to_string())
    })
    .await;

    match decode_result {
        Ok(Ok(img)) => TileState::Loaded(img),
        Ok(Err(e)) => {
            warn!("Failed to decode heightmap tile {zoom}/{x}/{y}: {e}");
            TileState::Failed(Instant::now())
        }
        Err(e) => {
            error!("Decode task for heightmap tile {zoom}/{x}/{y} panicked: {e}");
            TileState::Failed(Instant::now())
        }
    }
}

/// Height at one exact pixel of one tile, if loaded
fn sample_tile_pixel(
    cache: &TileCache,
    zoom: u8,
    tile_x: u32,
    tile_y: u32,
    px: u32,
    py: u32,
) -> Option<f32> {
    let slot = cache.get(&(zoom, tile_x, tile_y))?;
    match slot.value().get()? {
        TileState::Loaded(img) => Some(decode_elevation(img[(px, py)])),
        TileState::NoData => Some(0.0),
        TileState::Failed(_) => None,
    }
}

const TILE_PX: u32 = 512;

fn decode_elevation(pixel: image::Rgb<u8>) -> f32 {
    let r = pixel[0] as f32;
    let g = pixel[1] as f32;
    let b = pixel[2] as f32;
    (r * 256.0 + g + b / 256.0) - 32768.0
}

/// Bilinearly-sampled terrain height at `coord`. Bilinear sampling is needed
/// because the terrain isn't sampled pixel by pixel, but it's based off
/// coordinates which may result in uneven slopes if sampled by the nearest
/// pixel.
pub fn get_height_at_coord(coord: Coord, zoom: u8, cache: &TileCache) -> f32 {
    if coord.long < -180.0 || coord.long > 180.0 {
        return coord.long.clamp(-180.0, 180.0);
    }

    let n = 2.0_f32.powf(zoom as f32);
    let max_tile = (n as u32).saturating_sub(1);

    let x = n * ((coord.long + 180.0) / 360.0);
    let lat_rad = coord.lat.to_radians();
    let y = (1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / PI) / 2.0 * n;

    let tile_x = (x.floor() as i64).clamp(0, max_tile as i64) as u32;
    let tile_y = (y.floor() as i64).clamp(0, max_tile as i64) as u32;

    // In-tile pixel position, continuous.
    let px = ((x - x.floor()) * TILE_PX as f32).clamp(0.0, TILE_PX as f32);
    let py = ((y - y.floor()) * TILE_PX as f32).clamp(0.0, TILE_PX as f32);

    let px0 = (px.floor() as u32).min(TILE_PX - 1);
    let py0 = (py.floor() as u32).min(TILE_PX - 1);
    let fx = px - px0 as f32;
    let fy = py - py0 as f32;

    // The "+1" tap can spill into the next tile over - that neighbour is
    // guaranteed pre-fetched by calculate_required_tiles_for_chunk.
    let (tile_x1, px1) = if px0 + 1 >= TILE_PX {
        (tile_x.saturating_add(1).min(max_tile), 0)
    } else {
        (tile_x, px0 + 1)
    };
    let (tile_y1, py1) = if py0 + 1 >= TILE_PX {
        (tile_y.saturating_add(1).min(max_tile), 0)
    } else {
        (tile_y, py0 + 1)
    };

    let sample = |tx: u32, ty: u32, px: u32, py: u32| -> Option<f32> {
        sample_tile_pixel(cache, zoom, tx, ty, px, py)
    };

    let h00 = sample(tile_x, tile_y, px0, py0);
    let h10 = sample(tile_x1, tile_y, px1, py0);
    let h01 = sample(tile_x, tile_y1, px0, py1);
    let h11 = sample(tile_x1, tile_y1, px1, py1);

    match (h00, h10, h01, h11) {
        (Some(h00), Some(h10), Some(h01), Some(h11)) => {
            let top = h00 + (h10 - h00) * fx;
            let bottom = h01 + (h11 - h01) * fx;
            top + (bottom - top) * fy
        }
        _ => 0.0,
    }
}

// This plane mesh generation algorithm is based on the bevy Plane3d mesh generator
async fn build_mesh(
    normal: Dir3,
    chunk_translation: Vec3,
    chunk_size: f32,
    lod_level: u8,
    client: Client,
    semaphore: Arc<Semaphore>,
    cache: TileCache,
) -> Option<(Mesh, Collider)> {
    // Heightmap tiles are fetched at a zoom matching this node's own LOD level
    let heightmap_zoom = lod_level.min(MAX_HEIGHTMAP_ZOOM);
    let n = 2.0_f32.powf(heightmap_zoom as f32);
    let required_tiles = calculate_required_tiles_for_chunk(
        chunk_translation,
        chunk_size,
        normal,
        n,
        heightmap_zoom,
    );
    ensure_tiles_loaded(
        &client,
        Arc::clone(&semaphore),
        Arc::clone(&cache),
        required_tiles,
    )
    .await;

    let verts = SUBDIV_PER_TILE + 2;

    let chunk_translation_d = chunk_translation.as_dvec3();
    let rotation_d = DQuat::from_rotation_arc(DVec3::Y, (*normal).as_dvec3());
    let projected_chunk_center_d = to_sphere_pos(chunk_translation_d);
    let projected_chunk_center = projected_chunk_center_d.as_vec3();

    let vertex_count = (verts * verts) as usize;
    let mut positions = vec![Vec3::ZERO; vertex_count];
    let mut normals = vec![[0.0f32; 3]; vertex_count];
    let mut uvs = vec![[0.0f32; 2]; vertex_count];
    let mut heights = vec![[0.0f32; 2]; vertex_count];

    for z in 0..verts {
        for x in 0..verts {
            let idx = (z * verts + x) as usize;
            let tx = x as f64 / (verts - 1) as f64;
            let tz = z as f64 / (verts - 1) as f64;

            let even_spaced_pos =
                vertex_sphere_pos(rotation_d, chunk_translation_d, chunk_size, tx, tz);
            let coord = pos_to_coord(even_spaced_pos.as_vec3());

            let height = get_height_at_coord(coord, heightmap_zoom, &cache);
            let factor = 1.0 + height as f64 / EARTH_RADIUS as f64;

            let world_pos = even_spaced_pos * factor - projected_chunk_center_d;
            positions[idx] = world_pos.as_vec3();
            uvs[idx] = [tx as f32, tz as f32];
            heights[idx] = [height, 0.0];

            normals[idx] = sample_analytic_normal(coord, heightmap_zoom, &cache)
                .await
                .to_array();
        }
    }

    let mut indices: Vec<u32> = Vec::with_capacity(((verts - 1) * (verts - 1) * 6) as usize);
    for z in 0..verts - 1 {
        for x in 0..verts - 1 {
            let quad = z * verts + x;
            indices.push(quad + verts + 1);
            indices.push(quad + 1);
            indices.push(quad + verts);
            indices.push(quad);
            indices.push(quad + verts);
            indices.push(quad + 1);
        }
    }

    // Skirts help hide seams between different LOD levels
    let skirt_depth = chunk_size * EARTH_RADIUS * SKIRT_DEPTH_FACTOR;
    let top_row: Vec<u32> = (0..verts).collect();
    let bottom_row: Vec<u32> = (0..verts).map(|x| (verts - 1) * verts + x).collect();
    let left_col: Vec<u32> = (0..verts).map(|z| z * verts).collect();
    let right_col: Vec<u32> = (0..verts).map(|z| z * verts + verts - 1).collect();

    for edge in [top_row, bottom_row, left_col, right_col] {
        let mut top_dup = Vec::with_capacity(edge.len());
        let mut bottom = Vec::with_capacity(edge.len());

        for &vi in &edge {
            let p = positions[vi as usize];
            let n = normals[vi as usize];
            let uv = uvs[vi as usize];
            let h = heights[vi as usize];
            let inward = (p + projected_chunk_center).normalize() * -skirt_depth;

            positions.push(p);
            normals.push(n);
            uvs.push(uv);
            heights.push(h);
            top_dup.push(positions.len() as u32 - 1);

            positions.push(p + inward);
            normals.push(n);
            uvs.push(uv);
            heights.push(h);
            bottom.push(positions.len() as u32 - 1);
        }

        for i in 0..edge.len() - 1 {
            let top_a = top_dup[i];
            let top_b = top_dup[i + 1];
            let bot_a = bottom[i];
            let bot_b = bottom[i + 1];
            indices.extend_from_slice(&[top_a, bot_a, top_b, top_b, bot_a, bot_b]);
        }
    }

    let mut earth_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    earth_mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        positions.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
    );
    earth_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    earth_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    earth_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, heights);
    earth_mesh.insert_indices(Indices::U32(indices));

    let collider = Collider::trimesh_from_mesh(&earth_mesh)?;

    Some((earth_mesh, collider))
}

/// The sphere-warped world-plane position of one grid vertex within a
/// chunk, given its fractional position (`tx`, `tz` each in `0.0..=1.0`)
/// across the chunk.
fn vertex_sphere_pos(
    rotation_d: DQuat,
    chunk_translation_d: DVec3,
    chunk_size: f32,
    tx: f64,
    tz: f64,
) -> DVec3 {
    let local_pos = rotation_d
        * DVec3::new(
            (tx - 0.5) * chunk_size as f64,
            0.0,
            (tz - 0.5) * chunk_size as f64,
        );
    let world_plane_pos = local_pos + chunk_translation_d;
    to_sphere_pos(world_plane_pos)
}

/// The lat/long coordinate of one grid vertex - see `vertex_sphere_pos`.
fn vertex_coord(
    rotation_d: DQuat,
    chunk_translation_d: DVec3,
    chunk_size: f32,
    tx: f64,
    tz: f64,
) -> Coord {
    let pos = vertex_sphere_pos(rotation_d, chunk_translation_d, chunk_size, tx, tz);
    pos_to_coord(pos.as_vec3())
}

/// Adds the tile containing `coord`, plus its `(+1,0)`/`(0,+1)`/`(+1,+1)`
/// neighbours, to `tiles`. The neighbours cover whatever `get_height_at_coord`'s
/// bilinear taps might spill into when `coord` sits near the tile's far edge.
fn add_tile_and_bilinear_neighbors(
    tiles: &mut HashSet<(u8, u32, u32)>,
    coord: Coord,
    n: f32,
    zoom: u8,
    max_tile: u32,
) {
    let (tx, ty) = coord_to_tile(coord, n);
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        tiles.insert((zoom, (tx + dx).min(max_tile), (ty + dy).min(max_tile)));
    }
}

fn calculate_required_tiles_for_chunk(
    chunk_translation: Vec3,
    chunk_size: f32,
    normal: Dir3,
    n: f32,
    zoom: u8,
) -> Vec<(u8, u32, u32)> {
    let mut unique_tiles = HashSet::new();
    let max_tile_limit = (n as u32).saturating_sub(1);

    let rotation_d = DQuat::from_rotation_arc(DVec3::Y, (*normal).as_dvec3());
    let chunk_translation_d = chunk_translation.as_dvec3();

    let steps = SUBDIV_PER_TILE + 2;

    for i in 0..steps {
        for j in 0..steps {
            let tx = i as f64 / (steps - 1) as f64;
            let tz = j as f64 / (steps - 1) as f64;

            let coord = vertex_coord(rotation_d, chunk_translation_d, chunk_size, tx, tz);

            add_tile_and_bilinear_neighbors(&mut unique_tiles, coord, n, zoom, max_tile_limit);
            for offset_coord in normal_sample_coords(coord) {
                add_tile_and_bilinear_neighbors(
                    &mut unique_tiles,
                    offset_coord,
                    n,
                    zoom,
                    max_tile_limit,
                );
            }
        }
    }

    unique_tiles.into_iter().collect()
}

fn pos_to_coord(pos: Vec3) -> Coord {
    let distance_h = (pos.x.powi(2) + pos.z.powi(2)).sqrt();

    let bearing = pos.x.atan2(pos.z).to_degrees();

    let elevation = pos
        .y
        .atan2(distance_h)
        .to_degrees()
        .clamp(-85.05113, 85.05113);

    let coord = Coord {
        lat: elevation,
        long: bearing,
    };
    coord
}

/// What happened when trying to get one tile onto disk.
enum FetchOutcome {
    Downloaded,
    NoData,
}

async fn get_tile(
    client: &Client,
    semaphore: Arc<Semaphore>,
    coord: &TerrariumCoords,
) -> Result<FetchOutcome, String> {
    let file_name = format!(
        ".user/cache/elevation/{}_{}_{}.webp",
        coord.z, coord.x, coord.y
    );

    if Path::new(&file_name).exists() {
        return Ok(FetchOutcome::Downloaded);
    }

    let _permit = semaphore.acquire().await.map_err(|e| e.to_string())?;

    let url = format!(
        "https://tiles.mapterhorn.com/{}/{}/{}.webp",
        coord.z, coord.x, coord.y
    );

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(FetchOutcome::NoData);
    }

    if !response.status().is_success() {
        return Err(format!(
            "Server responded to {} with status: {}",
            &url,
            response.status()
        ));
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    if bytes.is_empty() {
        return Ok(FetchOutcome::NoData);
    }

    let mut file = File::create(&file_name).map_err(|e| e.to_string())?;
    file.write_all(&bytes).map_err(|e| e.to_string())?;

    Ok(FetchOutcome::Downloaded)
}

#[derive(Clone, Debug)]
pub struct TerrariumCoords {
    z: u8,
    x: u32,
    y: u32,
}

pub fn poll_terrain(
    mut tasks: Query<(Entity, &mut SpawnTerrain)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, TerrainMaterial>>>,
    asset_server: Res<AssetServer>,
    big_space: Single<Entity, With<BigSpace>>,
    mut registry: ResMut<TerrainChunkRegistry>,
) {
    for (entity, mut spawn_terrain) in &mut tasks {
        if let Some(mesh_collider) = future::block_on(future::poll_once(&mut spawn_terrain.task)) {
            registry.pending.remove(&spawn_terrain.key);

            if let Some((earth_mesh, collider)) = mesh_collider {
                let chunk = commands
                    .spawn((
                        TerrainFaces,
                        Mesh3d(meshes.add(earth_mesh)),
                        RigidBody::Static,
                        collider,
                        MeshMaterial3d(
                            terrain_materials.add(ExtendedMaterial {
                                base: StandardMaterial {
                                    base_color: Color::Srgba(GREEN),
                                    perceptual_roughness: 1.0,
                                    ..Default::default()
                                },
                                extension: TerrainMaterial {
                                    normals: asset_server
                                        .load_builder()
                                        .with_settings(|settings: &mut ImageLoaderSettings| {
                                            settings.is_srgb = false;
                                            settings.sampler =
                                                ImageSampler::Descriptor(ImageSamplerDescriptor {
                                                    address_mode_u: ImageAddressMode::Repeat,
                                                    address_mode_v: ImageAddressMode::Repeat,
                                                    mag_filter: ImageFilterMode::Linear,
                                                    min_filter: ImageFilterMode::Linear,
                                                    ..default()
                                                });
                                        })
                                        .load("textures/water_normals.png"),
                                    chunk_normal: absolute_position(
                                        &spawn_terrain.cell_coord,
                                        spawn_terrain.cell_offset,
                                    )
                                    .as_vec3(),
                                },
                            }),
                        ),
                        Transform::from_translation(spawn_terrain.cell_offset),
                        spawn_terrain.cell_coord,
                    ))
                    .id();

                commands.entity(*big_space).add_child(chunk);
                registry.live.insert(spawn_terrain.key, chunk);
            }

            commands.entity(entity).remove::<SpawnTerrain>();
        }
    }
}
