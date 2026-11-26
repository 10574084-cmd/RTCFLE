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
        "        let mut shared = Gc::write(mc, self.0.shared.get());",
        "        let changed = crate::vector_mutation::mutate_shape(&mut shared.shape, seed, intensity);",
        "        if changed > 0 {",
        "            shared.shape_bounds = shared.shape.shape_bounds;",
        "            shared.edge_bounds = shared.shape.edge_bounds;",
        "            shared.scaled_handle.borrow_mut().clear();",
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
        "        let mut shared = Gc::write(mc, self.0.shared.get());",
        "        let mut changed =",
        "            crate::vector_mutation::mutate_morph_shape(&mut shared.start, seed, intensity);",
        "        changed += crate::vector_mutation::mutate_morph_shape(&mut shared.end, seed, intensity);",
        "        if changed > 0 {",
        "            shared.frames.borrow_mut().clear();",
        "        }",
        "        changed",
        "    }",
        "",
        "    pub(crate) fn invalidate_vector_mutation_cache(self) {",
        "        if self.invalidate_cached_bitmap() {",
        "            let mut parent = self.parent();",
        "            while let Some(parent_object) = parent {",
        "                if !parent_object.invalidate_cached_bitmap() {",
        "                    break;",
        "                }",
        "                parent = parent_object.parent();",
        "            }",
        "        }",
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
        "            invalidate_vector_display_object(context.stage.into());"
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

console.log("Patched pinned Ruffle source for live in-place vector mutation.");
