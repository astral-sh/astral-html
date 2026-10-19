#!/usr/bin/env python3
"""Capture or replay a small public HTML corpus without committing the pages.

`capture` is intentionally explicit: it replaces manifest hashes for live pages.
`download` checks each fetched response against the checked-in manifest, and
reports content drift instead of silently comparing different input bytes.
"""

import argparse
import concurrent.futures
import datetime
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
PAGES = ROOT / ".corpus" / "pages"
SOURCES = ROOT / "corpus" / "sources.json"
MANIFEST = ROOT / "corpus" / "manifest.json"
MAX_BYTES = 3 * 1024 * 1024


def sources():
    source = json.loads(SOURCES.read_text())
    entries = [
        {"id": name, "url": url, "group": "live", "provenance": "public live HTML; site copyright applies"}
        for name, url in source["live"]
    ]
    for group, repository, revision, paths, license_name in (
        ("parse5", "inikulin/parse5", source["parse5_revision"], source["parse5_paths"], "MIT"),
        ("wpt", "web-platform-tests/wpt", source["wpt_revision"], source["wpt_paths"], "BSD-3-Clause"),
    ):
        for name, path in paths:
            entries.append({
                "id": f"{group}-{name}",
                "url": f"https://raw.githubusercontent.com/{repository}/{revision}/{path}",
                "group": group,
                "provenance": f"{repository}@{revision}:{path}",
                "license": license_name,
            })
    ids = [entry["id"] for entry in entries]
    if len(set(ids)) != len(ids):
        raise ValueError("duplicate corpus entry ID")
    return entries


def digest(data):
    return hashlib.sha256(data).hexdigest()


def fetch(entry):
    request = urllib.request.Request(entry["url"], headers={
        "User-Agent": "astral-html-public-corpus/1.0 (research; no cookies)",
        "Accept": "text/html,*/*;q=0.8",
        "Accept-Encoding": "identity",
    })
    with urllib.request.urlopen(request, timeout=35) as response:
        if response.status != 200:
            raise ValueError(f"HTTP {response.status}: {entry['url']}")
        content = response.read(MAX_BYTES + 1)
        if len(content) > MAX_BYTES:
            raise ValueError(f"exceeds {MAX_BYTES} byte limit: {entry['url']}")
        if not content:
            raise ValueError(f"empty response: {entry['url']}")
        metadata = {
            **entry,
            "file": f"{entry['id']}.html",
            "retrieved_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds"),
            "effective_url": response.url,
            "content_type": response.headers.get("Content-Type", ""),
            "bytes": len(content),
            "sha256": digest(content),
            "utf8_replacements": content.decode("utf-8", errors="replace").count("\ufffd"),
        }
        return metadata, content


def verify(entry):
    page = PAGES / entry["file"]
    if not page.is_file():
        return f"missing: {page}"
    content = page.read_bytes()
    if len(content) != entry["bytes"] or digest(content) != entry["sha256"]:
        return f"mismatch: {page} (the exact page used in the report is unavailable)"
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("capture", "download", "verify"))
    args = parser.parse_args()
    PAGES.mkdir(parents=True, exist_ok=True)

    if args.action == "capture":
        entries = sources()
        failures = []
        captured = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
            futures = {executor.submit(fetch, entry): entry for entry in entries}
            for future in concurrent.futures.as_completed(futures):
                entry = futures[future]
                try:
                    metadata, content = future.result()
                except Exception as error:
                    failures.append(f"{entry['id']}: {error}")
                    continue
                (PAGES / metadata["file"]).write_bytes(content)
                captured.append(metadata)
                print(f"{metadata['id']:40s} {metadata['bytes']:>9} bytes")
        for failure in failures:
            print(failure)
        if failures:
            raise SystemExit(f"{len(failures)} fetches failed; manifest was not changed")
        MANIFEST.write_text(json.dumps({
            "description": "Public HTML bytes captured for astral-html recovery comparisons",
            "encoding": "UTF-8 with replacement for malformed bytes; no network during evaluation",
            "entries": sorted(captured, key=lambda entry: entry["id"]),
        }, indent=2, ensure_ascii=False) + "\n")
    else:
        entries = json.loads(MANIFEST.read_text())["entries"]
        if args.action == "download":
            missing = [entry for entry in entries if verify(entry)]
            with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
                futures = {executor.submit(fetch, entry): entry for entry in missing}
                for future in concurrent.futures.as_completed(futures):
                    entry = futures[future]
                    metadata, content = future.result()
                    if metadata["sha256"] != entry["sha256"]:
                        print(f"drift: {entry['id']}: expected {entry['sha256']}, fetched {metadata['sha256']}")
                        continue
                    (PAGES / entry["file"]).write_bytes(content)
        failures = [failure for entry in entries if (failure := verify(entry))]
        for failure in failures:
            print(failure)
        if failures:
            raise SystemExit(f"{len(failures)} pages missing or changed; use capture to record a new corpus")
        print(f"Verified {len(entries)} pages, {sum(entry['bytes'] for entry in entries):,} bytes")


if __name__ == "__main__":
    main()
