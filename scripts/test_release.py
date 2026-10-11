import unittest
from pathlib import Path

import release


class Versions(unittest.TestCase):
    def test_versions_are_three_plain_numbers(self):
        self.assertEqual(release.parse_version("1.0.0"), (1, 0, 0))
        self.assertEqual(release.parse_version("10.20.30"), (10, 20, 30))
        for bad in ["", "1", "1.0", "1.0.0.0", "v1.0.0", "1.0.0-beta", "a.b.c", "1.0.x", " 1.0.0"]:
            self.assertIsNone(release.parse_version(bad), bad)

    def test_the_latest_tag_is_the_highest_version_not_the_newest_name(self):
        self.assertEqual(release.latest_version(["v1.2.0", "v1.10.0", "v1.9.9", "nightly", "v2.0"]), "v1.10.0")
        self.assertIsNone(release.latest_version([]))
        self.assertIsNone(release.latest_version(["latest", "v1", "release-1.0.0"]))


class Bumping(unittest.TestCase):
    def test_a_version_in_the_right_place_is_replaced(self):
        text = '{\n\t"name": "vrccreationapp",\n\t"version": "1.0.0",\n\t"dependencies": {"x": {"version": "1.0.0"}}\n}\n'
        pattern, count = release.FILES[0][1], release.FILES[0][2]
        updated = release.bump_text(text, "1.0.0", "1.0.1", pattern, count)
        self.assertIn('"name": "vrccreationapp",\n\t"version": "1.0.1"', updated)
        self.assertIn('{"version": "1.0.0"}', updated)

    def test_a_missing_or_extra_match_is_an_error(self):
        pattern, count = release.FILES[0][1], release.FILES[0][2]
        with self.assertRaises(ValueError):
            release.bump_text('{"name": "other", "version": "1.0.0"}', "1.0.0", "1.0.1", pattern, count)
        twice = '{"name": "vrccreationapp", "version": "1.0.0"} {"name": "vrccreationapp", "version": "1.0.0"}'
        with self.assertRaises(ValueError):
            release.bump_text(twice, "1.0.0", "1.0.1", pattern, count)

    def test_every_real_file_can_be_bumped_at_the_current_version(self):
        have = release.current_version()
        self.assertIsNotNone(release.parse_version(have))
        for name, pattern, count in release.FILES:
            text = (release.ROOT / name).read_text(encoding="utf-8")
            updated = release.bump_text(text, have, "9.9.9", pattern, count)
            self.assertEqual(updated.count("9.9.9") - text.count("9.9.9"), count, name)
            self.assertEqual(len(updated), len(text) + count * (len("9.9.9") - len(have)), name)

    def test_cargo_lock_only_changes_this_crate(self):
        text = 'name = "other"\nversion = "1.0.0"\n\n[[package]]\nname = "vrccreationapp"\nversion = "1.0.0"\n'
        updated = release.bump_text(text, "1.0.0", "1.0.1", release.FILES[3][1], release.FILES[3][2])
        self.assertIn('name = "other"\nversion = "1.0.0"', updated)
        self.assertIn('name = "vrccreationapp"\nversion = "1.0.1"', updated)


class Notes(unittest.TestCase):
    def test_release_commits_are_left_out_and_ids_follow_subjects(self):
        lines = release.commit_lines([("abc1234", "Add an update check"), ("def5678", "Release 1.0.0"), ("0000000", "  "), ("1111111", "Fix the thing")])
        self.assertEqual(lines, ["- Add an update check (abc1234)", "- Fix the thing (1111111)"])

    def test_notes_have_the_description_then_the_changes(self):
        notes = release.make_notes("My description", ["- One (abc1234)", "- Two (def5678)"])
        self.assertEqual(notes, "My description\n\n### Changes\n- One (abc1234)\n- Two (def5678)\n")

    def test_notes_without_a_description_or_commits(self):
        self.assertEqual(release.make_notes("", ["- One (abc1234)"]), "### Changes\n- One (abc1234)\n")
        self.assertEqual(release.make_notes("Hello", []), "Hello\n")
        self.assertEqual(release.make_notes("", [], "Fallback text"), "Fallback text\n")

    def test_the_first_release_can_use_the_changelog_section(self):
        text = "# Changelog\n\n## 1.0.0 (unreleased)\n\nFirst release.\n\n### Projects\n- A\n\n## 0.9.0\n\nOld.\n"
        self.assertEqual(release.changelog_section(text, "1.0.0"), "First release.\n\n### Projects\n- A")
        self.assertEqual(release.changelog_section(text, "0.9.0"), "Old.")
        self.assertEqual(release.changelog_section(text, "3.0.0"), "")


class Changelog(unittest.TestCase):
    def test_an_unreleased_heading_loses_the_label(self):
        text = "# Changelog\n\n## 1.0.0 (unreleased)\n\nBody\n"
        self.assertEqual(release.update_changelog(text, "1.0.0", "ignored"), "# Changelog\n\n## 1.0.0\n\nBody\n")

    def test_a_new_version_is_added_at_the_top(self):
        text = "# Changelog\n\n## 1.0.0\n\nBody\n"
        updated = release.update_changelog(text, "1.0.1", "Notes\n")
        self.assertTrue(updated.startswith("# Changelog\n\n## 1.0.1\n\nNotes\n\n## 1.0.0"))

    def test_a_version_that_is_already_listed_is_refused(self):
        with self.assertRaises(ValueError):
            release.update_changelog("# Changelog\n\n## 1.0.1\n\nBody\n", "1.0.1", "Notes")


if __name__ == "__main__":
    unittest.main()
