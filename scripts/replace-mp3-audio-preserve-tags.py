#!/usr/bin/env python3
"""Replace an MP3 audio payload while preserving target tag bytes.

This script treats an MP3 as:

    [optional ID3v2 tag][MPEG audio frames][optional ID3v1 tag]

It copies the tag regions from the target file byte-for-byte and replaces only
the audio region with the audio region from a source MP3.
"""

from __future__ import annotations

import argparse
from pathlib import Path


def syncsafe_to_int(value: bytes) -> int:
    if len(value) != 4:
        raise ValueError("syncsafe value must be 4 bytes")

    return (value[0] << 21) | (value[1] << 14) | (value[2] << 7) | value[3]


def id3v2_end(data: bytes) -> int:
    if data[:3] != b"ID3":
        return 0

    if len(data) < 10:
        raise ValueError("truncated ID3v2 header")

    flags = data[5]
    tag_size = syncsafe_to_int(data[6:10])
    end = 10 + tag_size

    if flags & 0x10:
        end += 10

    if end > len(data):
        raise ValueError("ID3v2 tag extends beyond file length")

    return end


def id3v1_start(data: bytes) -> int:
    if len(data) >= 128 and data[-128:-125] == b"TAG":
        return len(data) - 128

    return len(data)


def split_mp3(data: bytes) -> tuple[bytes, bytes, bytes]:
    front_tag_end = id3v2_end(data)
    rear_tag_start = id3v1_start(data)

    if front_tag_end > rear_tag_start:
        raise ValueError("tag regions overlap")

    return (
        data[:front_tag_end],
        data[front_tag_end:rear_tag_start],
        data[rear_tag_start:],
    )


def replace_audio(target: Path, source: Path, output: Path) -> None:
    target_data = target.read_bytes()
    source_data = source.read_bytes()

    target_front_tag, _, target_rear_tag = split_mp3(target_data)
    _, source_audio, _ = split_mp3(source_data)

    if not source_audio:
        raise ValueError(f"{source} has no audio payload")

    output.write_bytes(target_front_tag + source_audio + target_rear_tag)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Replace the MPEG audio frame region of an MP3 while preserving "
            "the target file's ID3 tag bytes."
        )
    )
    parser.add_argument("target", type=Path, help="MP3 whose tags should be preserved")
    parser.add_argument("source", type=Path, help="MP3 whose audio should be copied")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="output path; defaults to replacing the target in place",
    )

    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output = args.output if args.output is not None else args.target

    replace_audio(args.target, args.source, output)


if __name__ == "__main__":
    main()
