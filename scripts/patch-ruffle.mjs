import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(process.argv[2] || "ruffle-source");
const here = path.dirname(fileURLToPath(import.meta.url));
const tick = String.fromCharCode(96);

function patch(file, oldLines, newLines) {
    const filePath = path.join(root, file);
    const source = fs.readFileSync(filePath, "utf8");
    const oldText = oldLines.join("\n");
    const newText = newLines.join("\n");
    const first = source.indexOf(oldText);
    if (first < 0 || source.indexOf(oldText, first + oldText.length) >= 0) {
        throw new Error("Patch anchor missing or ambiguous in " + file + ": " + oldLines[0]);
    }
    fs.writeFileSync(filePath, source.slice(0, first) + newText + source.slice(first + oldText.length));
}

const vectorModule = path.join(root, "core/src/vector_mutation.rs");
if (fs.existsSync(vectorModule)) {
    throw new Error("core/src/vector_mutation.rs already exists; refusing to overwrite.");
}
fs.copyFileSync(path.join(here, "vector_mutation.rs"), vectorModule);

patch("core/src/lib.rs",
    ["mod types;", "mod vminterface;"],
    ["mod types;", "mod vector_mutation;", "mod vminterface;"]);

patch("core/src/tessellation_cache.rs",
    [
        "    pub(crate) fn new() -> Self {",
        "        Self {",
        "            entries: std::array::from_fn(|_| None),",
        "            len: 0,",
        "        }",
        "    }"
    ],
    [
        "    pub(crate) fn new() -> Self {",
        "        Self {",
        "            entries: std::array::from_fn(|_| None),",
        "            len: 0,",
        "        }",
        "    }",
        "",
        "    /// Drops every cached tessellation after the source vector shape changes.",
        "    pub(crate) fn clear(&mut self) {",
        "        self.entries = std::array::from_fn(|_| None);",
        "        self.len = 0;",
        "    }"
    ]);


patch("core/src/display_object/graphic.rs",
    ["use gc_arena::lock::Lock;"],
    ["use gc_arena::lock::{Lock, RefLock};"]);

patch("core/src/display_object/graphic.rs",
    ["            shape_bounds: swf_shape.shape_bounds,", "            edge_bounds: swf_shape.edge_bounds,", "            shape: swf_shape,"],
    ["            shape: RefLock::new(swf_shape),"]);

patch("core/src/display_object/graphic.rs",
    ["            shape_bounds: Default::default(),", "            edge_bounds: Default::default(),", "            shape: swf::Shape {"],
    ["            shape: RefLock::new(swf::Shape {"]);

patch("core/src/display_object/graphic.rs",
    ["                shape: Vec::new(),", "            },", "            movie: *context.root_swf,"],
    ["                shape: Vec::new(),", "            }),", "            movie: *context.root_swf,"]);

patch("core/src/display_object/graphic.rs",
    ["        let new_handle = context.renderer.register_shape_with_scale(", "            (&shared.shape).into(),"],
    ["        let shape = shared.shape.borrow();", "        let new_handle = context.renderer.register_shape_with_scale(", "            (&*shape).into(),"]);

patch("core/src/display_object/graphic.rs",
    ["        } else if include_strokes {", "            self.0.shared.get().shape_bounds", "        } else {", "            self.0.shared.get().edge_bounds", "        }"],
    ["        } else {", "            let shared = self.0.shared.get();", "            let shape = shared.shape.borrow();", "            if include_strokes {", "                shape.shape_bounds", "            } else {", "                shape.edge_bounds", "            }", "        }"]);

patch("core/src/display_object/graphic.rs",
    ["    #[collect(require_static)]", "    shape: swf::Shape,", "", "    #[collect(require_static)]", "    shape_bounds: Rectangle<Twips>,", "", "    #[collect(require_static)]", "    edge_bounds: Rectangle<Twips>,"],
    ["    #[collect(require_static)]", "    shape: RefLock<swf::Shape>,"]);

patch("core/src/display_object/graphic.rs",
    [
        "    pub fn drawing_mut(&self) -> RefMut<'_, Drawing> {",
        "        self.0.drawing.get_or_init(Default::default).borrow_mut()",
        "    }"
    ],
    [
        "    pub fn drawing_mut(&self) -> RefMut<'_, Drawing> {",
        "        self.0.drawing.get_or_init(Default::default).borrow_mut()",
        "    }",
        "",
        "    pub(crate) fn mutation_key(self) -> usize {",
        "        Gc::as_ptr(self.0.shared.get()) as usize",
        "    }",
        "",
        "    pub(crate) fn mutate_vectors(",
        "        self,",
        "        mc: &Mutation<'gc>,",
        "        seed: &mut u32,",
        "        intensity: u8,",
        "    ) -> u32 {",
        "        let changed = {",
        "            let mut shape = unlock!(Gc::write(mc, self.0.shared.get()), GraphicShared, shape).borrow_mut();",
        "            crate::vector_mutation::mutate_shape(&mut shape, seed, intensity)",
        "        };",
        "        if changed > 0 {",
        "            self.0.shared.get().scaled_handle.borrow_mut().clear();",
        "        }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);


patch("core/src/display_object/morph_shape.rs",
    ["use gc_arena::lock::Lock;"],
    ["use gc_arena::lock::{Lock, RefLock};"]);

patch("core/src/display_object/morph_shape.rs",
    ["            start: swf_tag.start.clone(),", "            end: swf_tag.end.clone(),"],
    ["            start: RefLock::new(swf_tag.start.clone()),", "            end: RefLock::new(swf_tag.end.clone()),"]);

patch("core/src/display_object/morph_shape.rs",
    ["    #[collect(require_static)]", "    start: swf::MorphShape,", "", "    #[collect(require_static)]", "    end: swf::MorphShape,"],
    ["    #[collect(require_static)]", "    start: RefLock<swf::MorphShape>,", "", "    #[collect(require_static)]", "    end: RefLock<swf::MorphShape>,"]);

patch("core/src/display_object/morph_shape.rs",
    ["            BoundsMode::Script => shared.start.shape_bounds,", "", "            // For getRect(), use the `edge_bounds` that were declared to be the", "            // bounds of the start shape", "            BoundsMode::ScriptWithoutStrokes => shared.start.edge_bounds,"],
    ["            BoundsMode::Script => shared.start.borrow().shape_bounds,", "", "            // For getRect() or hitTestObject(), use the `edge_bounds` that were declared to be the", "            // bounds of the start shape", "            BoundsMode::ScriptWithoutStrokes => shared.start.borrow().edge_bounds,"]);

patch("core/src/display_object/morph_shape.rs",
    ["        use swf::{FillStyle, LineStyle, ShapeRecord, ShapeStyles};", "        // Start shape is ratio 65535, end shape is ratio 0."],
    ["        use swf::{FillStyle, LineStyle, ShapeRecord, ShapeStyles};", "        let start_morph = self.start.borrow();", "        let end_morph = self.end.borrow();", "        // Start shape is ratio 65535, end shape is ratio 0."]);

patch("core/src/display_object/morph_shape.rs",
    ["        let fill_styles: Vec<FillStyle> = self", "            .start", "            .fill_styles", "            .iter()", "            .zip(self.end.fill_styles.iter())"],
    ["        let fill_styles: Vec<FillStyle> = start_morph", "            .fill_styles", "            .iter()", "            .zip(end_morph.fill_styles.iter())"]);

patch("core/src/display_object/morph_shape.rs",
    ["        let line_styles: Vec<LineStyle> = self", "            .start", "            .line_styles", "            .iter()", "            .zip(self.end.line_styles.iter())"],
    ["        let line_styles: Vec<LineStyle> = start_morph", "            .line_styles", "            .iter()", "            .zip(end_morph.line_styles.iter())"]);

patch("core/src/display_object/morph_shape.rs",
    ["        let mut shape = Vec::with_capacity(self.start.shape.len());", "        let mut start_iter = self.start.shape.iter();", "        let mut end_iter = self.end.shape.iter();"],
    ["        let mut shape = Vec::with_capacity(start_morph.shape.len());", "        let mut start_iter = start_morph.shape.iter();", "        let mut end_iter = end_morph.shape.iter();"]);

patch("core/src/display_object/morph_shape.rs",
    [
        "    pub fn instantiate(self, gc_context: &Mutation<'gc>) -> Self {",
        "        Self(Gc::new(gc_context, (*self.0).clone()))",
        "    }"
    ],
    [
        "    pub fn instantiate(self, gc_context: &Mutation<'gc>) -> Self {",
        "        Self(Gc::new(gc_context, (*self.0).clone()))",
        "    }",
        "",
        "    pub(crate) fn mutation_key(self) -> usize {",
        "        Gc::as_ptr(self.0.shared.get()) as usize",
        "    }",
        "",
        "    pub(crate) fn mutate_vectors(",
        "        self,",
        "        mc: &Mutation<'gc>,",
        "        seed: &mut u32,",
        "        intensity: u8,",
        "    ) -> u32 {",
        "        let mut changed = {",
        "            let mut start = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, start).borrow_mut();",
        "            crate::vector_mutation::mutate_morph_shape(&mut start, seed, intensity)",
        "        };",
        "        changed += {",
        "            let mut end = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, end).borrow_mut();",
        "            crate::vector_mutation::mutate_morph_shape(&mut end, seed, intensity)",
        "        };",
        "        if changed > 0 {",
        "            self.0.shared.get().frames.borrow_mut().clear();",
        "        }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/display_object.rs",
    [
        "        pub fn as_movie_clip for MovieClip;",
        "        pub fn as_edit_text for EditText;"
    ],
    [
        "        pub fn as_movie_clip for MovieClip;",
        "        pub fn as_graphic for Graphic;",
        "        pub fn as_edit_text for EditText;"
    ]);

patch("core/src/player.rs",
    ["use crate::compatibility_rules::UrlRewriteStage;"],
    ["use crate::character::Character;", "use crate::compatibility_rules::UrlRewriteStage;"]);

patch("core/src/player.rs",
    [
        "impl Player {",
        "    // This method will panic if called inside an " + tick + "enter_arena_mut" + tick + " call."
    ],
    [
        "fn invalidate_vector_display_object<'gc>(object: DisplayObject<'gc>) {",
        "    if let Some(graphic) = object.as_graphic() {",
        "        graphic.invalidate_vector_mutation_cache();",
        "    }",
        "    if let Some(morph) = object.as_morph_shape() {",
        "        morph.invalidate_vector_mutation_cache();",
        "    }",
        "    if let Some(container) = object.as_container() {",
        "        for child in container.iter_render_list() {",
        "            invalidate_vector_display_object(child);",
        "        }",
        "    }",
        "}",
        "",
        "impl Player {",
        "    // This method will panic if called inside an " + tick + "enter_arena_mut" + tick + " call."
    ]);

patch("core/src/player.rs",
    [
        "    /// Runs the closure " + tick + "f" + tick + " with an " + tick + "UpdateContext" + tick + ".",
        "    /// This takes cares of populating the " + tick + "UpdateContext" + tick + " struct, avoiding borrow issues.",
        "    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"
    ],
    [
        "    /// Mutates every vector definition registered in the loaded SWF libraries.",
        "    /// Active instances are invalidated in place; the movie is never reloaded.",
        "    pub fn mutate_vector_shapes(&mut self, intensity: u8, seed: u32) -> u32 {",
        "        self.mutate_with_update_context(|context| {",
        "            let mut characters = Vec::new();",
        "            let movies = context.library.known_movies().collect::<Vec<_>>();",
        "            for movie in movies {",
        "                if let Some(movie_library) = context.library.library_for_movie(movie) {",
        "                    characters.extend(movie_library.characters().values().copied());",
        "                }",
        "            }",
        "",
        "            let mut seen = FnvHashSet::default();",
        "            let mut seed = seed;",
        "            let mut changed = 0;",
        "            let intensity = intensity.clamp(1, 100);",
        "            for character in characters {",
        "                match character {",
        "                    Character::Graphic(graphic) => {",
        "                        if seen.insert(graphic.mutation_key()) {",
        "                            changed += graphic.mutate_vectors(context.gc(), &mut seed, intensity);",
        "                        }",
        "                    }",
        "                    Character::MorphShape(morph) => {",
        "                        if seen.insert(morph.mutation_key()) {",
        "                            changed += morph.mutate_vectors(context.gc(), &mut seed, intensity);",
        "                        }",
        "                    }",
        "                    _ => {}",
        "                }",
        "            }",
        "",
        "            invalidate_vector_display_object(context.stage.into());",
        "            if changed > 0 {",
        "                *context.needs_render = true;",
        "            }",
        "            changed",
        "        })",
        "    }",
        "",
        "    /// Runs the closure " + tick + "f" + tick + " with an " + tick + "UpdateContext" + tick + ".",
        "    /// This takes cares of populating the " + tick + "UpdateContext" + tick + " struct, avoiding borrow issues.",
        "    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"
    ]);

patch("web/src/lib.rs",
    [
        "    pub fn set_volume(&self, value: f32) {",
        "        let _ = self.with_core_mut(|core| core.set_volume(value));",
        "    }"
    ],
    [
        "    pub fn set_volume(&self, value: f32) {",
        "        let _ = self.with_core_mut(|core| core.set_volume(value));",
        "    }",
        "",
        "    /// Applies an in-place vector mutation to live display objects without reloading the SWF.",
        "    pub fn mutate_vector_shapes(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_shapes(intensity, seed))",
        "            .unwrap_or_default()",
        "    }"
    ]);

patch("web/packages/core/src/public/player/v1.ts",
    ["    resume(): void;"],
    [
        "    resume(): void;",
        "",
        "    /**",
        "     * Mutates vector shapes already loaded in the live Ruffle display list.",
        "     * Does not call load(), reload(), suspend(), or resume().",
        "     * @param intensity Mutation strength from 1 to 100.",
        "     * @param seed Deterministic seed for this mutation pass.",
        "     * @returns Number of vector fields changed.",
        "     */",
        "    mutateVectorShapes(intensity: number, seed: number): number;"
    ]);

patch("web/packages/core/src/internal/player/impl_v1.ts",
    [
        "    resume(): void {",
        "        this.#inner.play();",
        "    }"
    ],
    [
        "    resume(): void {",
        "        this.#inner.play();",
        "    }",
        "",
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorShapes(intensity, seed);",
        "    }"
    ]);

patch("web/packages/core/src/internal/player/inner.tsx",
    [
        "    play(): void {",
        "        if (this.instance) {",
        "            this.instance.play();",
        "            this.playButton.style.display = \"none\";",
        "        }",
        "    }"
    ],
    [
        "    play(): void {",
        "        if (this.instance) {",
        "            this.instance.play();",
        "            this.playButton.style.display = \"none\";",
        "        }",
        "    }",
        "",
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_shapes(safeIntensity, seed >>> 0);",
        "    }"
    ]);


patch("core/src/display_object/graphic.rs",
    ["        } else if !self.0.shared.get().shape.shape.is_empty() {"],
    ["        } else if !self.0.shared.get().shape.borrow().shape.is_empty() {"]);

patch("core/src/display_object/graphic.rs",
    [
        "                let shape = &self.0.shared.get().shape;",
        "                return ruffle_render::shape_utils::shape_hit_test(shape, point, &local_matrix);"
    ],
    [
        "                let shape = self.0.shared.get().shape.borrow();",
        "                return ruffle_render::shape_utils::shape_hit_test(&*shape, point, &local_matrix);"
    ]);


patch("core/src/display_object/graphic.rs",
    [
        "                let shape = self.0.shared.get().shape.borrow();",
        "                return ruffle_render::shape_utils::shape_hit_test(&*shape, point, &local_matrix);"
    ],
    [
        "                let shared = self.0.shared.get();",
        "                let shape = shared.shape.borrow();",
        "                return ruffle_render::shape_utils::shape_hit_test(&*shape, point, &local_matrix);"
    ]);



// Separate high-amplitude vector stretching path; the classic mutation API stays intact.
patch("core/src/display_object/graphic.rs",
    [
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    }",
        "",
        "    pub(crate) fn mutate_stretch_vectors(",
        "        self,",
        "        mc: &Mutation<'gc>,",
        "        seed: &mut u32,",
        "        intensity: u8,",
        "    ) -> u32 {",
        "        let changed = {",
        "            let mut shape = unlock!(Gc::write(mc, self.0.shared.get()), GraphicShared, shape).borrow_mut();",
        "            crate::vector_mutation::mutate_stretch_shape(&mut shape, seed, intensity)",
        "        };",
        "        if changed > 0 {",
        "            self.0.shared.get().scaled_handle.borrow_mut().clear();",
        "        }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/display_object/morph_shape.rs",
    [
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    }",
        "",
        "    pub(crate) fn mutate_stretch_vectors(",
        "        self,",
        "        mc: &Mutation<'gc>,",
        "        seed: &mut u32,",
        "        intensity: u8,",
        "    ) -> u32 {",
        "        let mut changed = {",
        "            let mut start = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, start).borrow_mut();",
        "            crate::vector_mutation::mutate_stretch_morph_shape(&mut start, seed, intensity)",
        "        };",
        "        changed += {",
        "            let mut end = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, end).borrow_mut();",
        "            crate::vector_mutation::mutate_stretch_morph_shape(&mut end, seed, intensity)",
        "        };",
        "        if changed > 0 {",
        "            self.0.shared.get().frames.borrow_mut().clear();",
        "        }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/player.rs",
    ["    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"],
    [
        "    /// Applies high-amplitude vector stretching to loaded shapes without restarting the movie.",
        "    pub fn mutate_vector_stretch(&mut self, intensity: u8, seed: u32) -> u32 {",
        "        self.mutate_with_update_context(|context| {",
        "            let mut characters = Vec::new();",
        "            let movies = context.library.known_movies().collect::<Vec<_>>();",
        "            for movie in movies {",
        "                if let Some(movie_library) = context.library.library_for_movie(movie) {",
        "                    characters.extend(movie_library.characters().values().copied());",
        "                }",
        "            }",
        "            let mut seen = FnvHashSet::default();",
        "            let mut seed = seed;",
        "            let mut changed = 0;",
        "            let intensity = intensity.clamp(1, 100);",
        "            for character in characters {",
        "                match character {",
        "                    Character::Graphic(graphic) => {",
        "                        if seen.insert(graphic.mutation_key()) {",
        "                            changed += graphic.mutate_stretch_vectors(context.gc(), &mut seed, intensity);",
        "                        }",
        "                    }",
        "                    Character::MorphShape(morph) => {",
        "                        if seen.insert(morph.mutation_key()) {",
        "                            changed += morph.mutate_stretch_vectors(context.gc(), &mut seed, intensity);",
        "                        }",
        "                    }",
        "                    _ => {}",
        "                }",
        "            }",
        "            invalidate_vector_display_object(context.stage.into());",
        "            if changed > 0 {",
        "                *context.needs_render = true;",
        "            }",
        "            changed",
        "        })",
        "    }",
        "",
        "    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"
    ]);

patch("web/src/lib.rs",
    [
        "    pub fn mutate_vector_shapes(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_shapes(intensity, seed))",
        "            .unwrap_or_default()",
        "    }"
    ],
    [
        "    pub fn mutate_vector_shapes(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_shapes(intensity, seed))",
        "            .unwrap_or_default()",
        "    }",
        "",
        "    /// Applies in-place high-amplitude vector stretching to the live movie.",
        "    pub fn mutate_vector_stretch(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_stretch(intensity, seed))",
        "            .unwrap_or_default()",
        "    }"
    ]);

patch("web/packages/core/src/public/player/v1.ts",
    ["    mutateVectorShapes(intensity: number, seed: number): number;"],
    [
        "    mutateVectorShapes(intensity: number, seed: number): number;",
        "    /** Applies high-amplitude vector stretching without reloading the movie. */",
        "    mutateVectorStretch(intensity: number, seed: number): number;"
    ]);

patch("web/packages/core/src/internal/player/impl_v1.ts",
    [
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorShapes(intensity, seed);",
        "    }"
    ],
    [
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorShapes(intensity, seed);",
        "    }",
        "",
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorStretch(intensity, seed);",
        "    }"
    ]);

patch("web/packages/core/src/internal/player/inner.tsx",
    [
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_shapes(safeIntensity, seed >>> 0);",
        "    }"
    ],
    [
        "    mutateVectorShapes(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_shapes(safeIntensity, seed >>> 0);",
        "    }",
        "",
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_stretch(safeIntensity, seed >>> 0);",
        "    }"
    ]);




// Live instance-level aspect-ratio warp. This edits the current display list,
// not shared Graphic definitions, so Classic Mutation and Vector Stretch remain independent.
patch("core/src/player.rs",
    ["fn invalidate_vector_display_object<'gc>(object: DisplayObject<'gc>) {"],
    [
        "fn mutate_sprite_aspect_display_object<'gc>(",
        "    object: DisplayObject<'gc>,",
        "    seed: &mut u32,",
        "    intensity: u8,",
        "    glitch_chance: u8,",
        ") -> u32 {",
        "    let mut changed = 0;",
        "    if object.as_stage().is_none() {",
        "        let mut matrix = object.base().matrix();",
        "        if crate::vector_mutation::mutate_sprite_aspect_matrix(",
        "            &mut matrix, seed, intensity, glitch_chance,",
        "        ) {",
        "            object.set_matrix(matrix);",
        "            object.invalidate_cached_bitmap();",
        "            changed += 1;",
        "        }",
        "    }",
        "    if let Some(container) = object.as_container() {",
        "        for child in container.iter_render_list() {",
        "            changed += mutate_sprite_aspect_display_object(",
        "                child, seed, intensity, glitch_chance,",
        "            );",
        "        }",
        "    }",
        "    changed",
        "}",
        "",
        "fn invalidate_vector_display_object<'gc>(object: DisplayObject<'gc>) {"
    ]);

patch("core/src/player.rs",
    ["    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"],
    [
        "    /// Mutates live display-object instance matrices to produce wide/tall sprite aspect warps.",
        "    /// This does not rewrite shared shapes or reload the SWF.",
        "    pub fn mutate_sprite_aspect_warp(",
        "        &mut self,",
        "        intensity: u8,",
        "        glitch_chance: u8,",
        "        seed: u32,",
        "    ) -> u32 {",
        "        self.mutate_with_update_context(|context| {",
        "            let mut seed = seed;",
        "            let changed = mutate_sprite_aspect_display_object(",
        "                context.stage.into(), &mut seed, intensity, glitch_chance,",
        "            );",
        "            if changed > 0 {",
        "                *context.needs_render = true;",
        "            }",
        "            changed",
        "        })",
        "    }",
        "",
        "    pub fn mutate_with_update_context<F, R>(&mut self, f: F) -> R"
    ]);

patch("web/src/lib.rs",
    [
        "    pub fn mutate_vector_stretch(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_stretch(intensity, seed))",
        "            .unwrap_or_default()",
        "    }"
    ],
    [
        "    pub fn mutate_vector_stretch(&self, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_stretch(intensity, seed))",
        "            .unwrap_or_default()",
        "    }",
        "",
        "    /// Applies aspect-ratio and matrix glitches to the current live sprite instances.",
        "    pub fn mutate_sprite_aspect_warp(&self, intensity: u8, glitch_chance: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_sprite_aspect_warp(intensity, glitch_chance, seed))",
        "            .unwrap_or_default()",
        "    }"
    ]);

patch("web/packages/core/src/public/player/v1.ts",
    ["    mutateVectorStretch(intensity: number, seed: number): number;"],
    [
        "    mutateVectorStretch(intensity: number, seed: number): number;",
        "    /** Randomly stretches live sprite instance aspect ratios and adds optional matrix glitches. */",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number;"
    ]);

patch("web/packages/core/src/internal/player/impl_v1.ts",
    [
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorStretch(intensity, seed);",
        "    }"
    ],
    [
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorStretch(intensity, seed);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {",
        "        return this.#inner.mutateSpriteAspectWarp(intensity, glitchChance, seed);",
        "    }"
    ]);

patch("web/packages/core/src/internal/player/inner.tsx",
    [
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_stretch(safeIntensity, seed >>> 0);",
        "    }"
    ],
    [
        "    mutateVectorStretch(intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_stretch(safeIntensity, seed >>> 0);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        const safeGlitchChance = Math.max(0, Math.min(100, Math.round(glitchChance)));",
        "        return this.instance.mutate_sprite_aspect_warp(safeIntensity, safeGlitchChance, seed >>> 0);",
        "    }"
    ]);




// Expose the twenty vector corruption algorithms as one dedicated runtime API.
patch("core/src/display_object/graphic.rs",
    [
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    pub(crate) fn mutate_vector_effects(",
        "        self, mc: &Mutation<'gc>, seed: &mut u32, intensity: u8, mode: u8,",
        "    ) -> u32 {",
        "        let changed = {",
        "            let mut shape = unlock!(Gc::write(mc, self.0.shared.get()), GraphicShared, shape).borrow_mut();",
        "            crate::vector_mutation::mutate_vector_effect_shape(&mut shape, seed, intensity, mode)",
        "        };",
        "        if changed > 0 { self.0.shared.get().scaled_handle.borrow_mut().clear(); }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/display_object/morph_shape.rs",
    [
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    pub(crate) fn mutate_vector_effects(",
        "        self, mc: &Mutation<'gc>, seed: &mut u32, intensity: u8, mode: u8,",
        "    ) -> u32 {",
        "        let mut changed = {",
        "            let mut start = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, start).borrow_mut();",
        "            crate::vector_mutation::mutate_vector_effect_morph_shape(&mut start, seed, intensity, mode)",
        "        };",
        "        changed += {",
        "            let mut end = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, end).borrow_mut();",
        "            crate::vector_mutation::mutate_vector_effect_morph_shape(&mut end, seed, intensity, mode)",
        "        };",
        "        if changed > 0 { self.0.shared.get().frames.borrow_mut().clear(); }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/player.rs",
    ["    pub fn mutate_sprite_aspect_warp("],
    [
        "    /// Applies one of twenty dedicated vector effects, or all twenty when mode is 20.",
        "    pub fn mutate_vector_effects(&mut self, mode: u8, intensity: u8, seed: u32) -> u32 {",
        "        self.mutate_with_update_context(|context| {",
        "            let mut characters = Vec::new();",
        "            let movies = context.library.known_movies().collect::<Vec<_>>();",
        "            for movie in movies {",
        "                if let Some(movie_library) = context.library.library_for_movie(movie) {",
        "                    characters.extend(movie_library.characters().values().copied());",
        "                }",
        "            }",
        "            let mut seen = FnvHashSet::default();",
        "            let mut seed = seed;",
        "            let mut changed = 0;",
        "            let intensity = intensity.clamp(1, 100);",
        "            for character in characters {",
        "                match character {",
        "                    Character::Graphic(graphic) => {",
        "                        if seen.insert(graphic.mutation_key()) {",
        "                            changed += graphic.mutate_vector_effects(context.gc(), &mut seed, intensity, mode);",
        "                        }",
        "                    }",
        "                    Character::MorphShape(morph) => {",
        "                        if seen.insert(morph.mutation_key()) {",
        "                            changed += morph.mutate_vector_effects(context.gc(), &mut seed, intensity, mode);",
        "                        }",
        "                    }",
        "                    _ => {}",
        "                }",
        "            }",
        "            invalidate_vector_display_object(context.stage.into());",
        "            if changed > 0 { *context.needs_render = true; }",
        "            changed",
        "        })",
        "    }",
        "",
        "    pub fn mutate_sprite_aspect_warp("
    ]);

patch("web/src/lib.rs",
    ["    pub fn mutate_sprite_aspect_warp(&self, intensity: u8, glitch_chance: u8, seed: u32) -> u32 {"],
    [
        "    pub fn mutate_vector_effects(&self, mode: u8, intensity: u8, seed: u32) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_effects(mode, intensity, seed)).unwrap_or_default()",
        "    }",
        "",
        "    pub fn mutate_sprite_aspect_warp(&self, intensity: u8, glitch_chance: u8, seed: u32) -> u32 {"
    ]);

patch("web/packages/core/src/public/player/v1.ts",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number;"],
    [
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number;",
        "    /** Runs one of twenty vector effects (0-19), or every vector effect (20). */",
        "    mutateVectorEffects(mode: number, intensity: number, seed: number): number;"
    ]);

patch("web/packages/core/src/internal/player/impl_v1.ts",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"],
    [
        "    mutateVectorEffects(mode: number, intensity: number, seed: number): number {",
        "        return this.#inner.mutateVectorEffects(mode, intensity, seed);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"
    ]);

patch("web/packages/core/src/internal/player/inner.tsx",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"],
    [
        "    mutateVectorEffects(mode: number, intensity: number, seed: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeMode = Math.max(0, Math.min(20, Math.round(mode)));",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        return this.instance.mutate_vector_effects(safeMode, safeIntensity, seed >>> 0);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"
    ]);



// Per-mode vector appearance switches: independently preserve/corrupt colors
// and optionally apply a posterized cartoon palette with heavier outlines.
patch("core/src/display_object/graphic.rs",
    [
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    pub(crate) fn mutate_with_appearance_options(",
        "        self, mc: &Mutation<'gc>, seed: &mut u32, intensity: u8, kind: u8, mode: u8,",
        "        corrupt_colors: bool, cartoonize: bool, cartoon_strength: u8,",
        "    ) -> u32 {",
        "        let changed = {",
        "            let mut shape = unlock!(Gc::write(mc, self.0.shared.get()), GraphicShared, shape).borrow_mut();",
        "            match kind {",
        "                0 => crate::vector_mutation::mutate_shape_with_options(&mut shape, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                1 => crate::vector_mutation::mutate_stretch_shape_with_options(&mut shape, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                _ => crate::vector_mutation::mutate_vector_effect_shape_with_options(&mut shape, seed, intensity, mode, corrupt_colors, cartoonize, cartoon_strength),",
        "            }",
        "        };",
        "        if changed > 0 { self.0.shared.get().scaled_handle.borrow_mut().clear(); }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/display_object/morph_shape.rs",
    [
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ],
    [
        "    pub(crate) fn mutate_with_appearance_options(",
        "        self, mc: &Mutation<'gc>, seed: &mut u32, intensity: u8, kind: u8, mode: u8,",
        "        corrupt_colors: bool, cartoonize: bool, cartoon_strength: u8,",
        "    ) -> u32 {",
        "        let mut changed = {",
        "            let mut start = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, start).borrow_mut();",
        "            match kind {",
        "                0 => crate::vector_mutation::mutate_morph_shape_with_options(&mut start, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                1 => crate::vector_mutation::mutate_stretch_morph_shape_with_options(&mut start, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                _ => crate::vector_mutation::mutate_vector_effect_morph_shape_with_options(&mut start, seed, intensity, mode, corrupt_colors, cartoonize, cartoon_strength),",
        "            }",
        "        };",
        "        changed += {",
        "            let mut end = unlock!(Gc::write(mc, self.0.shared.get()), MorphShapeShared, end).borrow_mut();",
        "            match kind {",
        "                0 => crate::vector_mutation::mutate_morph_shape_with_options(&mut end, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                1 => crate::vector_mutation::mutate_stretch_morph_shape_with_options(&mut end, seed, intensity, corrupt_colors, cartoonize, cartoon_strength),",
        "                _ => crate::vector_mutation::mutate_vector_effect_morph_shape_with_options(&mut end, seed, intensity, mode, corrupt_colors, cartoonize, cartoon_strength),",
        "            }",
        "        };",
        "        if changed > 0 { self.0.shared.get().frames.borrow_mut().clear(); }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        self.invalidate_cached_bitmap();",
        "    }"
    ]);

patch("core/src/player.rs",
    ["    pub fn mutate_sprite_aspect_warp("],
    [
        "    /// Runs Classic Mutation, Vector Stretch, or a dedicated vector effect with appearance options.",
        "    /// kind: 0 = classic, 1 = stretch, 2 = dedicated effect; modes 0-19 are selectable and 20 runs all.",
        "    pub fn mutate_vector_runtime_with_options(",
        "        &mut self, kind: u8, mode: u8, intensity: u8, seed: u32,",
        "        corrupt_colors: bool, cartoonize: bool, cartoon_strength: u8,",
        "    ) -> u32 {",
        "        self.mutate_with_update_context(|context| {",
        "            let mut characters = Vec::new();",
        "            let movies = context.library.known_movies().collect::<Vec<_>>();",
        "            for movie in movies {",
        "                if let Some(movie_library) = context.library.library_for_movie(movie) {",
        "                    characters.extend(movie_library.characters().values().copied());",
        "                }",
        "            }",
        "            let mut seen = FnvHashSet::default();",
        "            let mut seed = seed;",
        "            let mut changed = 0;",
        "            let intensity = intensity.clamp(1, 100);",
        "            let kind = kind.min(2);",
        "            for character in characters {",
        "                match character {",
        "                    Character::Graphic(graphic) => {",
        "                        if seen.insert(graphic.mutation_key()) {",
        "                            changed += graphic.mutate_with_appearance_options(context.gc(), &mut seed, intensity, kind, mode, corrupt_colors, cartoonize, cartoon_strength);",
        "                        }",
        "                    }",
        "                    Character::MorphShape(morph) => {",
        "                        if seen.insert(morph.mutation_key()) {",
        "                            changed += morph.mutate_with_appearance_options(context.gc(), &mut seed, intensity, kind, mode, corrupt_colors, cartoonize, cartoon_strength);",
        "                        }",
        "                    }",
        "                    _ => {}",
        "                }",
        "            }",
        "            invalidate_vector_display_object(context.stage.into());",
        "            if changed > 0 { *context.needs_render = true; }",
        "            changed",
        "        })",
        "    }",
        "",
        "    pub fn mutate_sprite_aspect_warp("
    ]);

patch("web/src/lib.rs",
    ["    pub fn mutate_sprite_aspect_warp(&self, intensity: u8, glitch_chance: u8, seed: u32) -> u32 {"],
    [
        "    /// Mutates live vector definitions with optional color preservation and cartoon styling.",
        "    pub fn mutate_vector_runtime_with_options(",
        "        &self, kind: u8, mode: u8, intensity: u8, seed: u32,",
        "        corrupt_colors: bool, cartoonize: bool, cartoon_strength: u8,",
        "    ) -> u32 {",
        "        self.with_core_mut(|core| core.mutate_vector_runtime_with_options(kind, mode, intensity, seed, corrupt_colors, cartoonize, cartoon_strength)).unwrap_or_default()",
        "    }",
        "",
        "    pub fn mutate_sprite_aspect_warp(&self, intensity: u8, glitch_chance: u8, seed: u32) -> u32 {"
    ]);

patch("web/packages/core/src/public/player/v1.ts",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number;"],
    [
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number;",
        "    /** Applies a vector mutation mode with optional color corruption and cartoon styling. */",
        "    mutateVectorRuntimeWithOptions(kind: number, mode: number, intensity: number, seed: number, corruptColors: boolean, cartoonize: boolean, cartoonStrength: number): number;"
    ]);

patch("web/packages/core/src/internal/player/impl_v1.ts",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"],
    [
        "    mutateVectorRuntimeWithOptions(kind: number, mode: number, intensity: number, seed: number, corruptColors: boolean, cartoonize: boolean, cartoonStrength: number): number {",
        "        return this.#inner.mutateVectorRuntimeWithOptions(kind, mode, intensity, seed, corruptColors, cartoonize, cartoonStrength);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"
    ]);

patch("web/packages/core/src/internal/player/inner.tsx",
    ["    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"],
    [
        "    mutateVectorRuntimeWithOptions(kind: number, mode: number, intensity: number, seed: number, corruptColors: boolean, cartoonize: boolean, cartoonStrength: number): number {",
        "        if (!this.instance) return 0;",
        "        const safeKind = Math.max(0, Math.min(2, Math.round(kind)));",
        "        const safeMode = Math.max(0, Math.min(20, Math.round(mode)));",
        "        const safeIntensity = Math.max(1, Math.min(100, Math.round(intensity)));",
        "        const safeCartoonStrength = Math.max(1, Math.min(100, Math.round(cartoonStrength)));",
        "        return this.instance.mutate_vector_runtime_with_options(safeKind, safeMode, safeIntensity, seed >>> 0, Boolean(corruptColors), Boolean(cartoonize), safeCartoonStrength);",
        "    }",
        "",
        "    mutateSpriteAspectWarp(intensity: number, glitchChance: number, seed: number): number {"
    ]);



// Expose Ruffle's actual WebAudio output as a MediaStream so preview recordings
// can include SWF audio without trying to tap the device speakers.
patch("web/Cargo.toml",
    ['    "TextMetrics", "OffscreenCanvasRenderingContext2d"'],
    ['    "TextMetrics", "OffscreenCanvasRenderingContext2d", "MediaStream", "MediaStreamAudioDestinationNode"']);

patch("web/src/audio.rs",
    [
        "use web_sys::{AudioContext, AudioContextOptions, AudioScheduledSourceNode};"
    ],
    [
        "use web_sys::{AudioContext, AudioContextOptions, AudioScheduledSourceNode, MediaStreamAudioDestinationNode};"
    ]);

patch("web/src/audio.rs",
    [
        "    context: AudioContext,",
        "    /// The current length of both buffers, in frames (pairs of left/right samples)."
    ],
    [
        "    context: AudioContext,",
        "    recording_destination: MediaStreamAudioDestinationNode,",
        "    /// The current length of both buffers, in frames (pairs of left/right samples)."
    ]);

patch("web/src/audio.rs",
    [
        "        let context = AudioContext::new_with_context_options(&opts).into_js_result()?;",
        "        let sample_rate = context.sample_rate();"
    ],
    [
        "        let context = AudioContext::new_with_context_options(&opts).into_js_result()?;",
        "        let recording_destination = context.create_media_stream_destination().into_js_result()?;",
        "        let _ = js_sys::Reflect::set(",
        "            &js_sys::global(),",
        "            &JsValue::from_str(\"__RTCFLE_AUDIO_STREAM\"),",
        "            &recording_destination.stream(),",
        "        );",
        "        let sample_rate = context.sample_rate();"
    ]);

patch("web/src/audio.rs",
    [
        "            context,",
        "            mixer: AudioMixer::new(2, sample_rate as u32),"
    ],
    [
        "            context,",
        "            recording_destination,",
        "            mixer: AudioMixer::new(2, sample_rate as u32),"
    ]);

patch("web/src/audio.rs",
    [
        "    context: AudioContext,",
        "    mixer_proxy: AudioMixerProxy,"
    ],
    [
        "    context: AudioContext,",
        "    recording_destination: MediaStreamAudioDestinationNode,",
        "    mixer_proxy: AudioMixerProxy,"
    ]);

patch("web/src/audio.rs",
    [
        "            context: audio.context.clone(),",
        "            mixer_proxy: audio.mixer.proxy(),"
    ],
    [
        "            context: audio.context.clone(),",
        "            recording_destination: audio.recording_destination.clone(),",
        "            mixer_proxy: audio.mixer.proxy(),"
    ]);

patch("web/src/audio.rs",
    [
        "        audio_node",
        "            .connect_with_audio_node(&self.context.destination())",
        "            .into_js_result()?;",
        "        let scheduled: &AudioScheduledSourceNode = &audio_node;"
    ],
    [
        "        audio_node",
        "            .connect_with_audio_node(&self.context.destination())",
        "            .into_js_result()?;",
        "        audio_node",
        "            .connect_with_audio_node(&self.recording_destination)",
        "            .into_js_result()?;",
        "        let scheduled: &AudioScheduledSourceNode = &audio_node;"
    ]);

console.log("Patched pinned Ruffle source for classic mutation, Vector Stretch, Sprite Aspect Warp, twenty vector effects, and live appearance controls.");

