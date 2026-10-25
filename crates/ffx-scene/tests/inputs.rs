//! The scene inputs of a capture (`ffx_scene::inputs`) against the
//! captured draws (`docs/formats/scene.md`, "Scene inputs"): the camera
//! and the world matrices the hook read at `renderWorld` reproduce
//! `scene.View`, `World` and `WorldViewProjection` bit for bit.
//!
//! Needs `FFX_SCENE_INPUTS` (one or more `capture.ffxgfx` files of
//! `tools/replays/draw-field.txt`'s cdsp0000 frames, separated by `:`);
//! skips when it is unset. With `FFX_GAME_DIR` each map draw is tied to
//! its own instance through the census (`World` of exactly that
//! instance); without it, to any instance with an equal `World`.

use std::collections::BTreeMap;

use ffx_scene::assets::{AssetIndex, TQ_LOW_DROPPED_LEVELS, is_shader_cluster};
use ffx_scene::capture::Capture;
use ffx_scene::census;
use ffx_scene::inputs::{self, SceneInputs};

fn captures() -> Vec<std::path::PathBuf> {
    std::env::var_os("FFX_SCENE_INPUTS")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default()
}

fn index() -> Option<AssetIndex> {
    let game = std::env::var_os("FFX_GAME_DIR")?;
    let archive = ffx_vbf::Archive::open(std::path::Path::new(&game).join("data/FFX_Data.vbf"))
        .expect("archive");
    let mut index = AssetIndex::new(TQ_LOW_DROPPED_LEVELS);
    index
        .add_archive(&archive, |p| {
            is_shader_cluster(p) || p.contains("/map/cdsp/cdsp00/")
        })
        .expect("index");
    Some(index)
}

#[derive(Default, Debug)]
struct Tally {
    checked: usize,
    exact: usize,
    notes: Vec<String>,
}

impl Tally {
    fn check(
        &mut self,
        what: &str,
        draw: usize,
        got: Option<inputs::Mat>,
        want: Option<inputs::Mat>,
    ) {
        let (Some(g), Some(w)) = (got, want) else {
            return;
        };
        self.checked += 1;
        if g.iter().zip(&w).all(|(a, b)| a.to_bits() == b.to_bits()) {
            self.exact += 1;
        } else if self.notes.len() < 8 {
            self.notes.push(format!(
                "{what} draw {draw}: {} ulp\n  captured {g:?}\n  inputs   {w:?}",
                census::ulps(&g, &w)
            ));
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)] // one walk over the frame's draws
fn inputs_reproduce_the_captured_matrices() {
    let files = captures();
    if files.is_empty() {
        eprintln!("FFX_SCENE_INPUTS unset: skipped");
        return;
    }
    let index = index();
    for file in files {
        let bytes = std::fs::read(&file).expect("capture");
        let cap = Capture::read(&bytes).expect("capture");
        let all = inputs::read(&bytes).expect("inputs");
        let frame = cap.frame.number;
        let list: Vec<&SceneInputs> = all
            .iter()
            .filter(|s| s.frame == frame && s.world == 0xac)
            .collect();
        assert!(
            !list.is_empty(),
            "{}: no map inputs in frame {frame}; records {:?}",
            file.display(),
            all.iter()
                .map(|s| (s.frame, s.world, &s.pass))
                .collect::<Vec<_>>()
        );
        for s in &list {
            eprintln!("{}", inputs::summary(s).lines().next().unwrap_or(""));
            assert!(!s.bounds.is_empty(), "no bounds");
            assert!(
                s.instances.iter().all(|i| i.cluster != u32::MAX),
                "unresolved instances"
            );
        }
        let map = inputs::cluster_named(list[0], "cdsp00").expect("the cdsp00 cluster");
        let draws = index.as_ref().map(|ix| census::census(&cap, ix));
        let file_instances = index
            .as_ref()
            .and_then(|ix| ix.instances.iter().find(|(m, _)| m.contains("cdsp00")))
            .map(|(m, v)| (m.clone(), v.clone()));
        let mut tally: BTreeMap<&str, Tally> = BTreeMap::new();
        let mut drawn_slots = std::collections::BTreeSet::new();
        let mut twins = 0;
        let mut scene_vp = None;
        let mut scene_no = 0usize;
        for (n, (_, d)) in cap.draws().enumerate() {
            if Capture::scene_targets(d).is_none() {
                continue;
            }
            let this_scene = scene_no;
            scene_no += 1;
            // The map's opaque pass (the camera and the worlds are the
            // same for all of the update's calls).
            let s = list
                .iter()
                .find(|s| s.pass.starts_with("Opaque"))
                .unwrap_or(&list[0]);
            let view = inputs::view(s);
            let vp = inputs::view_projection(s);
            tally.entry("scene.View").or_default().check(
                "scene.View",
                n,
                inputs::draw_matrix(&cap, d, "scene.View"),
                view,
            );
            tally.entry("scene.Projection").or_default().check(
                "scene.Projection",
                n,
                inputs::draw_matrix(&cap, d, "scene.Projection"),
                inputs::projection(s),
            );
            tally.entry("scene.ViewProjection").or_default().check(
                "scene.ViewProjection",
                n,
                inputs::draw_matrix(&cap, d, "scene.ViewProjection"),
                vp,
            );
            let Some(world) = inputs::draw_matrix(&cap, d, "World") else {
                continue;
            };
            // Which snapshot instance this draw is: by the census (the
            // file instance of its vertex data) or, without the archive,
            // by an equal World.
            let candidates: Vec<&inputs::Instance> = match (&draws, &file_instances) {
                (Some(draws), Some((model, file))) => {
                    let Some(c) = draws.iter().find(|c| c.number == this_scene) else {
                        continue;
                    };
                    if c.segment.as_ref().is_none_or(|(m, _)| m != model) {
                        continue;
                    }
                    // Every file instance of those names (two instances
                    // can share a name and a mesh: then World decides).
                    c.instances
                        .iter()
                        .flat_map(|name| {
                            file.iter()
                                .enumerate()
                                .filter(move |(_, f)| &f.name == name)
                                .map(|(slot, _)| slot)
                        })
                        .filter_map(|slot| {
                            s.instances.iter().find(|i| {
                                i.cluster == map && i.slot == u32::try_from(slot).unwrap_or(0)
                            })
                        })
                        .collect()
                }
                _ => s
                    .instances
                    .iter()
                    .filter(|i| {
                        inputs::world(i.world.as_ref())
                            .iter()
                            .zip(&world)
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                    })
                    .collect(),
            };
            if candidates.is_empty() {
                continue;
            }
            // With several candidates (a segment shared by instances),
            // the one whose World matches. The census ties a draw to a
            // segment by its vertex data, so of two instances with
            // separate but equal meshes (cdsp00's FBXASC056_0Shape and
            // FBXASC057_0Shape, mirrored) it names the first: then an
            // instance with as many segments and this World is its twin.
            let same_world = |i: &&inputs::Instance| {
                inputs::world(i.world.as_ref())
                    .iter()
                    .zip(&world)
                    .all(|(a, b)| a.to_bits() == b.to_bits())
            };
            let inst = match candidates.iter().copied().find(same_world) {
                Some(i) => i,
                None => {
                    let twin = s.instances.iter().find(|i| {
                        i.cluster == map
                            && i.segments.len() == candidates[0].segments.len()
                            && same_world(i)
                    });
                    if twin.is_some() {
                        twins += 1;
                    }
                    twin.unwrap_or(candidates[0])
                }
            };
            let w = inputs::world(inst.world.as_ref());
            drawn_slots.insert(inst.slot);
            scene_vp = scene_vp.or_else(|| inputs::draw_matrix(&cap, d, "scene.ViewProjection"));
            if w.iter()
                .zip(&world)
                .any(|(a, b)| a.to_bits() != b.to_bits())
            {
                let census_names = draws
                    .as_ref()
                    .and_then(|ds| ds.iter().find(|c| c.number == this_scene))
                    .map(|c| (c.segment.clone(), c.instances.clone()));
                let matching: Vec<u32> = s
                    .instances
                    .iter()
                    .filter(|i| {
                        inputs::world(i.world.as_ref())
                            .iter()
                            .zip(&world)
                            .all(|(a, b)| a.to_bits() == b.to_bits())
                    })
                    .map(|i| i.slot)
                    .collect();
                eprintln!(
                    "draw {n}: census {census_names:?}, candidate slots {:?}, slots with this World {matching:?}",
                    candidates.iter().map(|i| i.slot).collect::<Vec<_>>()
                );
            }
            tally
                .entry("World")
                .or_default()
                .check("World", n, Some(world), Some(w));
            tally.entry("WorldViewProjection").or_default().check(
                "WorldViewProjection",
                n,
                inputs::draw_matrix(&cap, d, "WorldViewProjection"),
                vp.map(|vp| inputs::world_view_projection(&vp, &w)),
            );
            tally.entry("WorldView").or_default().check(
                "WorldView",
                n,
                inputs::draw_matrix(&cap, d, "WorldView"),
                view.map(|v| inputs::world_view(&v, &w)),
            );
        }
        eprintln!(
            "{} ({}):",
            file.display(),
            if index.is_some() {
                "draws tied by the census"
            } else {
                "draws tied by an equal World"
            }
        );
        for (name, t) in &tally {
            eprintln!("  {name}: {}/{} exact", t.exact, t.checked);
            for note in &t.notes {
                eprintln!("    {note}");
            }
        }
        // Culling with each instance's own World: every map instance that
        // drew passes the box test, and the ones that did not are named.
        let s = list
            .iter()
            .find(|s| s.pass.starts_with("Opaque"))
            .unwrap_or(&list[0]);
        if let Some(vp) = scene_vp {
            let name = |slot: u32| {
                file_instances
                    .as_ref()
                    .and_then(|(_, f)| f.get(slot as usize))
                    .map_or_else(String::new, |f| f.name.clone())
            };
            let mut not_drawn = Vec::new();
            for b in &s.bounds {
                let Some(i) = inputs::bounds_instance(s, b) else {
                    not_drawn.push(format!("bounds {} (no instance)", b.slot));
                    continue;
                };
                let fb = ffx_phyre::Bounds {
                    min: b.min,
                    size: b.size,
                    render_order: b.render_order,
                };
                let visible = census::box_visible(&fb, &inputs::world(b.world.as_ref()), &vp);
                let hidden = i.segments.iter().all(|g| g.hidden != 0);
                if drawn_slots.contains(&i.slot) {
                    assert!(
                        visible,
                        "slot {} {} drew but fails the box test",
                        i.slot,
                        name(i.slot)
                    );
                } else {
                    not_drawn.push(format!(
                        "{} {} ({}{})",
                        i.slot,
                        name(i.slot),
                        if visible {
                            "passes the box test"
                        } else {
                            "outside the frustum"
                        },
                        if hidden { ", all segments hidden" } else { "" }
                    ));
                }
            }
            eprintln!(
                "  map instances drawn {} ({twins} draws tied to a twin), not drawn: {not_drawn:#?}",
                drawn_slots.len()
            );
        }
        for name in ["scene.View", "World", "WorldViewProjection"] {
            let t = tally.get(name).expect(name);
            assert!(t.checked > 0, "{name}: nothing checked");
            assert_eq!(t.exact, t.checked, "{name}: {:#?}", t.notes);
        }
        let world = &tally["World"];
        assert!(world.checked >= 50, "map draws tied: {}", world.checked);
    }
}
