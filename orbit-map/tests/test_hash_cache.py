from pathlib import Path

from orbit_map.pipeline.hash import detect_changes, save_hash_cache


def test_save_hash_cache_writes_into_knowledge_root(tmp_path: Path):
    output_dir = tmp_path / "knowledge"

    save_hash_cache({"a.py": "abc"}, output_dir)

    assert (output_dir / "hashes.json").exists()


def test_detect_changes_reads_hash_cache_from_knowledge_root(tmp_path: Path):
    output_dir = tmp_path / "knowledge"

    save_hash_cache({"a.py": "abc"}, output_dir)

    assert detect_changes({"a.py": "abc"}, output_dir) == []
    assert detect_changes({"a.py": "xyz"}, output_dir) == ["a.py"]
