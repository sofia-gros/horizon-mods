"""Uninstalled proposal over page facts, not native Text/Font/UI execution."""
from itertools import product
import hashlib
import json
from pathlib import Path
import subprocess
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PIN = json.loads((HERE / "modifier-anchor-oracle.json").read_text())
ENTRIES = PIN["modifier_entries"]
INDEX = "modifiers/index.json"
TARGETS = tuple(e["path"] for e in ENTRIES if e["path"] != INDEX)
TITLES = {e["path"]: e["title"] for e in ENTRIES}


def assemble(outputs, notices=(), source_titles=None):
    """Model completed source chunks; no splitting or title normalization.

    A tuple is (exact relative data path, sequence of first-span text facts).
    A None fact denotes a generated page without a first span. Empty chunks do
    not acquire an anchor. A nonempty source title must equal the first generated
    span. Only these six reviewed source paths are indexed.
    A later notice prepend shifts offsets, rather than masquerading as a source.
    """
    source_titles = TITLES if source_titles is None else source_titles
    outputs = tuple(outputs)
    if len(outputs) > PIN["limits"]["entries"]:
        raise ValueError("fixture entries")
    if len(notices) > PIN["limits"]["pages"]:
        raise ValueError("fixture pages")
    pages, anchors = [], {}
    for path, chunk in outputs:
        chunk = tuple(chunk)
        if len(pages) + len(chunk) + len(notices) > PIN["limits"]["pages"]:
            raise ValueError("fixture pages")
        title = source_titles.get(path)
        if chunk and path in TITLES and title and chunk[0] == title:
            anchors.setdefault(path, len(pages))
        pages.extend(chunk)
    return tuple(notices) + tuple(pages), {
        path: offset + len(notices) for path, offset in anchors.items()
    }


def first(pages, title):
    # The native predicate requires a first span; a missing title never matches.
    if title is None:
        return None
    return next((i for i, text in enumerate(pages) if text is not None and text == title), None)


def legacy(heading, modifier_label, pages, source_titles, first_page):
    """Independent translation of the pinned old predicates over supplied facts."""
    if modifier_label is None or heading != modifier_label:
        return None
    index = first(pages, source_titles.get(INDEX))
    if index is None:
        return None
    links = []
    for path in TARGETS:
        title = source_titles.get(path)
        offset = first(pages, title)
        if title is not None and offset is not None:
            links.append((title, first_page + offset))
    return index, tuple(links)


def proposed(chapter_key, anchors, source_titles, first_page, modifier_label="Modifiers"):
    """Candidate identity rule only. This is not installed in source_pages.

    Preserve source-title admission/labels; do not infer page identity from them.
    The native integration must keep checked integer conversion/addition.
    """
    if chapter_key != "modifiers" or modifier_label is None or not source_titles.get(INDEX):
        return None
    index = anchors.get(INDEX)
    if index is None:
        return None
    links = []
    for path in TARGETS:
        title, offset = source_titles.get(path), anchors.get(path)
        if title and offset is not None:
            links.append((title, first_page + offset))
    return index, tuple(links)


def outputs(counts=(1, 1, 1, 1, 1, 1), titles=None):
    titles = TITLES if titles is None else titles
    return [(entry["path"], (titles[entry["path"]],) * count)
            for entry, count in zip(ENTRIES, counts)]


class ModifierAnchorOracle(unittest.TestCase):
    def test_exact_source_pin_and_six_entry_resource_shape(self):
        bodies = {}
        for source in PIN["source_hashes"]:
            # This is the old oracle, not an assertion that candidate source is
            # unchanged. Separate inverse guards verify the candidate boundary.
            body = subprocess.check_output(
                ["git", "show", PIN["source_commit"] + ":" + source["path"]], cwd=ROOT)
            self.assertEqual(hashlib.sha256(body).hexdigest(), source["sha256"], source["path"])
            bodies[source["path"]] = body
        root = "tconstruct/content/assets/tconstruct/book/"
        owners = [book for book in PIN["books"] if any(
            c.get("name") == "modifiers" for c in json.loads(bodies[root + book + "/index.json"]))]
        self.assertEqual(owners, ["materials_and_you"])
        section = json.loads(bodies[root + "materials_and_you/sections/modifiers.json"])
        self.assertEqual([p["data"] for p in section], [p["path"] for p in ENTRIES])
        labels = dict(line.split("=", 1) for line in bodies[
            root + "materials_and_you/en_us/language.lang"].decode().splitlines() if "=" in line)
        self.assertEqual(labels["modifiers"], "Modifiers")
        self.assertEqual(len(ENTRIES), 6)
        self.assertEqual(len(set(TITLES.values())), 6)
        self.assertTrue(all(TITLES.values()))
        for entry in ENTRIES:
            value = json.loads(bodies[root + "materials_and_you/en_us/" + entry["path"]])
            self.assertEqual(value["title"], entry["title"])
        source = bodies["tconstruct/mixins/tconstruct/source-materials-book.rs"].decode()
        self.assertEqual(source.count(PIN["legacy_resolver_source"]), 1)

    def test_fixed_six_titles_have_equal_links_for_729_count_vectors(self):
        for counts in product(range(1, 4), repeat=6):
            pages, anchors = assemble(outputs(counts))
            self.assertEqual(legacy("Modifiers", "Modifiers", pages, TITLES, 7),
                             proposed("modifiers", anchors, TITLES, 7), counts)

    def test_duplicate_display_titles_are_intentional_different_destinations(self):
        titles = dict(TITLES)
        titles[TARGETS[0]] = titles[TARGETS[1]] = "Same translated title"
        pages, anchors = assemble(outputs(titles=titles), source_titles=titles)
        old = legacy("Modifiers", "Modifiers", pages, titles, 7)
        new = proposed("modifiers", anchors, titles, 7)
        self.assertEqual([p for _, p in old[1]][:2], [8, 8])
        self.assertEqual([p for _, p in new[1]][:2], [8, 9])

    def test_same_title_as_index_does_not_select_the_index_as_a_target(self):
        titles = dict(TITLES)
        titles[TARGETS[0]] = titles[INDEX]
        pages, anchors = assemble(outputs(titles=titles), source_titles=titles)
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, titles, 7)[1][0][1], 7)
        self.assertEqual(proposed("modifiers", anchors, titles, 7)[1][0][1], 8)

    def test_chapter_key_is_independent_of_heading_and_label_collisions(self):
        pages, anchors = assemble(outputs())
        self.assertIsNotNone(legacy("Modifiers", "Modifiers", pages, TITLES, 7))
        self.assertIsNone(proposed("other", anchors, TITLES, 7))
        self.assertIsNone(legacy("Translated heading", "Modifiers", pages, TITLES, 7))
        self.assertIsNotNone(proposed("modifiers", anchors, TITLES, 7))
        self.assertIsNone(proposed("Modifiers", anchors, TITLES, 7))
        self.assertIsNone(proposed("modifiers", anchors, TITLES, 7, modifier_label=None))

    def test_duplicate_data_path_keeps_first_successful_generated_chunk(self):
        chunks = [(TARGETS[0], ())] + outputs() + [(TARGETS[0], ("later",) * 3)]
        pages, anchors = assemble(chunks)
        self.assertEqual(anchors[TARGETS[0]], 1)
        self.assertEqual(proposed("modifiers", anchors, TITLES, 7)[1][0][1], 8)
        self.assertEqual(len(pages), 9)

    def test_zero_page_source_cannot_borrow_another_sources_equal_title(self):
        chunks = outputs((1, 0, 1, 1, 1, 1))
        chunks.insert(1, ("unrelated.json", (TITLES[TARGETS[0]],)))
        pages, anchors = assemble(chunks)
        self.assertEqual(len(legacy("Modifiers", "Modifiers", pages, TITLES, 7)[1]), 5)
        self.assertEqual(len(proposed("modifiers", anchors, TITLES, 7)[1]), 4)
        self.assertNotIn(TARGETS[0], anchors)

    def test_missing_source_title_and_missing_index_still_admit_no_link(self):
        pages, anchors = assemble(outputs())
        titles = dict(TITLES)
        del titles[TARGETS[0]]
        self.assertEqual(len(proposed("modifiers", anchors, titles, 7)[1]), 4)
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, titles, 7),
                         proposed("modifiers", anchors, titles, 7))
        del titles[INDEX]
        self.assertIsNone(proposed("modifiers", anchors, titles, 7))
        self.assertIsNone(legacy("Modifiers", "Modifiers", pages, titles, 7))

    def test_notice_prepend_shifts_offsets_and_cannot_impersonate_a_source(self):
        plain, before = assemble(outputs())
        pages, anchors = assemble(outputs(), ("Notice", "Notice continued"))
        self.assertEqual(pages[2:], plain)
        self.assertEqual(anchors, {p: n + 2 for p, n in before.items()})
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, TITLES, 7),
                         proposed("modifiers", anchors, TITLES, 7))
        pages, anchors = assemble(outputs(), (TITLES[INDEX],))
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, TITLES, 7)[0], 0)
        self.assertEqual(proposed("modifiers", anchors, TITLES, 7)[0], 1)

    def test_graphic_then_overflow_links_first_page_without_new_font_queries(self):
        pages, anchors = assemble(outputs((1, 2, 4, 1, 1, 1)))
        self.assertEqual(anchors[TARGETS[1]], 3)
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, TITLES, 7),
                         proposed("modifiers", anchors, TITLES, 7))

    def test_exact_paths_and_pages_without_spans_are_not_title_aliases(self):
        chunks = outputs()
        chunks[1] = (TARGETS[0], (None,))
        chunks.insert(1, ("other/traits.json", (TITLES[TARGETS[0]],)))
        pages, anchors = assemble(chunks)
        self.assertNotIn(TARGETS[0], anchors)
        self.assertEqual(legacy("Modifiers", "Modifiers", pages, TITLES, 7)[1][0][1], 8)
        self.assertEqual(len(proposed("modifiers", anchors, TITLES, 7)[1]), 4)

    def test_empty_source_title_is_not_a_linkable_anchor(self):
        titles = dict(TITLES)
        titles[TARGETS[0]] = ""
        pages, anchors = assemble(outputs(titles=titles), source_titles=titles)
        self.assertNotIn(TARGETS[0], anchors)
        self.assertEqual(len(legacy("Modifiers", "Modifiers", pages, titles, 7)[1]), 5)
        self.assertEqual(len(proposed("modifiers", anchors, titles, 7)[1]), 4)

    def test_oracle_is_bounded_and_not_a_runtime_page_limit(self):
        with self.assertRaises(ValueError):
            assemble([(INDEX, ())] * 33)
        with self.assertRaises(ValueError):
            assemble([(INDEX, ("page",) * 257)])
        with self.assertRaises(ValueError):
            assemble([], ("notice",) * 257)


if __name__ == "__main__":
    unittest.main()
