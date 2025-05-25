#!/usr/bin/env python3
"""Rust source code stripper for AI processing optimization.

This module provides functionality to strip Rust source code of documentation
comments while preserving all functional code and regular comments.
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Set, Union

from pydantic import BaseModel, Field, validator


class RustStripperError(Exception):
    """Base exception for Rust source code stripping operations."""
    pass


class FileProcessingError(RustStripperError):
    """Exception raised when file processing fails."""
    pass


class StripperConfig(BaseModel):
    """Configuration for the Rust source code stripper."""

    source_dir: Path = Field(default=Path("."))
    output_dir: Path = Field(default=Path("processed_project"))
    preserve_cargo_toml: bool = Field(default=True)
    preserve_readme: bool = Field(default=False)
    max_consecutive_blank_lines: int = Field(default=1, ge=0, le=3)
    file_extensions: Set[str] = Field(
        default_factory=lambda: {".rs", ".toml", ".md", ".txt"}
    )
    exclude_patterns: Set[str] = Field(
        default_factory=lambda: {"target/*", "*.lock", ".git/*", "processed_project/*"}
    )

    @validator("source_dir", "output_dir", pre=True)
    def validate_paths(cls, v: Union[str, Path]) -> Path:
        """Validate and convert path inputs to Path objects."""
        if isinstance(v, str):
            v = Path(v)
        return v.resolve()

    class Config:
        arbitrary_types_allowed = True


@dataclass
class ProcessingStats:
    """Statistics for file processing operations."""

    files_processed: int = 0
    files_skipped: int = 0
    files_failed: int = 0
    bytes_removed: int = 0
    lines_removed: int = 0
    errors: List[str] = field(default_factory=list)

    def add_error(self, error: str) -> None:
        """Add an error to the processing statistics."""
        self.errors.append(error)
        self.files_failed += 1


class RustSourceStripper:
    """Main class for stripping Rust source code files."""

    def __init__(self, config: StripperConfig) -> None:
        """Initialize the Rust source stripper."""
        self.config = config
        self.stats = ProcessingStats()

        # Simple patterns for whitespace cleanup
        self._trailing_whitespace_pattern = re.compile(r'[ \t]+$', re.MULTILINE)
        self._excessive_whitespace_pattern = re.compile(r'\n\s*\n\s*\n+')

    def process_project(self) -> ProcessingStats:
        """Process the entire Rust project."""
        try:
            self._create_output_directory()
            self._process_all_files()
            self._log_final_stats()
            return self.stats

        except Exception as e:
            error_msg = f"Failed to process project: {e}"
            print(f"ERROR: {error_msg}")
            raise FileProcessingError(error_msg) from e

    def _create_output_directory(self) -> None:
        """Create the output directory structure."""
        try:
            self.config.output_dir.mkdir(parents=True, exist_ok=True)
            print(f"Created output directory: {self.config.output_dir}")

        except OSError as e:
            raise FileProcessingError(f"Failed to create output directory: {e}") from e

    def _process_all_files(self) -> None:
        """Process all files in the source directory."""
        for file_path in self._get_files_to_process():
            try:
                self._process_single_file(file_path)

            except Exception as e:
                error_msg = f"Failed to process {file_path}: {e}"
                self.stats.add_error(error_msg)
                print(f"ERROR: {error_msg}")

    def _get_files_to_process(self) -> List[Path]:
        """Get list of files to process based on configuration."""
        files_to_process: List[Path] = []

        for file_path in self.config.source_dir.rglob("*"):
            if (
                file_path.is_file()
                and self._should_process_file(file_path)
                and not self._is_excluded(file_path)
            ):
                files_to_process.append(file_path)

        print(f"Found {len(files_to_process)} files to process")
        return files_to_process

    def _should_process_file(self, file_path: Path) -> bool:
        """Determine if a file should be processed."""
        # Check file extension
        if file_path.suffix not in self.config.file_extensions:
            return False

        # Special handling for specific files
        if file_path.name == "Cargo.toml" and not self.config.preserve_cargo_toml:
            return False

        if file_path.name.upper().startswith("README") and not self.config.preserve_readme:
            return False

        return True

    def _is_excluded(self, file_path: Path) -> bool:
        """Check if a file matches any exclusion patterns."""
        relative_path = file_path.relative_to(self.config.source_dir)

        for pattern in self.config.exclude_patterns:
            if relative_path.match(pattern):
                return True

        return False

    def _process_single_file(self, file_path: Path) -> None:
        """Process a single file."""
        try:
            # Read original file
            original_content = self._read_file_content(file_path)
            original_size = len(original_content)
            original_lines = original_content.count('\n')

            # Process content based on file type
            if file_path.suffix == ".rs":
                processed_content = self._strip_rust_file(original_content)
            else:
                processed_content = self._strip_generic_file(original_content)

            # Calculate statistics
            processed_size = len(processed_content)
            processed_lines = processed_content.count('\n')

            self.stats.bytes_removed += original_size - processed_size
            self.stats.lines_removed += original_lines - processed_lines

            # Write processed file
            self._write_processed_file(file_path, processed_content)

            self.stats.files_processed += 1

            if original_size > 0:
                reduction_percent = round((1 - processed_size / original_size) * 100, 2)
                print(f"Processed {file_path.name}: {original_size} -> {processed_size} bytes ({reduction_percent}% reduction)")

        except Exception as e:
            raise FileProcessingError(f"Failed to process {file_path}: {e}") from e

    def _read_file_content(self, file_path: Path) -> str:
        """Read content from a file with proper encoding handling."""
        try:
            # Try UTF-8 first, fallback to latin-1 for problematic files
            for encoding in ['utf-8', 'latin-1']:
                try:
                    return file_path.read_text(encoding=encoding)
                except UnicodeDecodeError:
                    continue

            raise FileProcessingError(f"Unable to decode file {file_path}")

        except OSError as e:
            raise FileProcessingError(f"Failed to read {file_path}: {e}") from e

    def _strip_rust_file(self, content: str) -> str:
        """Strip a Rust source file of ONLY documentation comments."""
        lines = content.split('\n')
        processed_lines = []
        in_doc_block = False

        for line in lines:
            stripped_line = line.strip()

            # Handle block doc comments
            if not in_doc_block:
                # Look for start of block doc comment
                doc_start = line.find('/**')
                if doc_start != -1:
                    # Check if it ends on the same line
                    doc_end = line.find('*/', doc_start + 3)
                    if doc_end != -1:
                        # Single line block doc comment - remove it
                        before = line[:doc_start]
                        after = line[doc_end + 2:]
                        new_line = before + after
                        if new_line.strip():
                            processed_lines.append(new_line)
                    else:
                        # Multi-line block doc comment starts
                        before = line[:doc_start]
                        if before.strip():
                            processed_lines.append(before)
                        in_doc_block = True
                elif stripped_line.startswith('///'):
                    # Line doc comment - skip it entirely
                    continue
                else:
                    # Regular line - keep it
                    processed_lines.append(line)
            else:
                # We're in a block doc comment
                doc_end = line.find('*/')
                if doc_end != -1:
                    # End of block doc comment
                    after = line[doc_end + 2:]
                    if after.strip():
                        processed_lines.append(after)
                    in_doc_block = False
                # Otherwise skip this line (it's part of the doc comment)

        # Join lines back together
        content = '\n'.join(processed_lines)

        # Clean up whitespace
        content = self._trailing_whitespace_pattern.sub('', content)
        content = self._normalize_blank_lines(content)
        content = content.strip()

        # Ensure file ends with newline
        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _strip_generic_file(self, content: str) -> str:
        """Strip a generic file of unnecessary whitespace."""
        # For non-Rust files, just normalize whitespace
        content = self._trailing_whitespace_pattern.sub('', content)
        content = self._normalize_blank_lines(content)
        content = content.strip()

        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _normalize_blank_lines(self, content: str) -> str:
        """Normalize excessive blank lines."""
        max_lines = self.config.max_consecutive_blank_lines
        replacement = '\n' * (max_lines + 1)

        return self._excessive_whitespace_pattern.sub(replacement, content)

    def _write_processed_file(self, original_path: Path, content: str) -> None:
        """Write processed content to output file."""
        try:
            # Calculate relative path from source directory
            relative_path = original_path.relative_to(self.config.source_dir)
            output_path = self.config.output_dir / relative_path

            # Create parent directories
            output_path.parent.mkdir(parents=True, exist_ok=True)

            # Write processed content
            output_path.write_text(content, encoding='utf-8')

        except OSError as e:
            raise FileProcessingError(f"Failed to write {output_path}: {e}") from e

    def _log_final_stats(self) -> None:
        """Log final processing statistics."""
        print(f"\nProcessing completed:")
        print(f"  Files processed: {self.stats.files_processed}")
        print(f"  Files skipped: {self.stats.files_skipped}")
        print(f"  Files failed: {self.stats.files_failed}")
        print(f"  Bytes removed: {self.stats.bytes_removed}")
        print(f"  Lines removed: {self.stats.lines_removed}")
        if self.stats.errors:
            print(f"  Errors: {len(self.stats.errors)}")


def main() -> int:
    """Main entry point for the script."""
    try:
        # Configure for current project structure
        config = StripperConfig(
            source_dir=Path("."),
            output_dir=Path("processed_project"),
            preserve_cargo_toml=True,
            preserve_readme=False,
            max_consecutive_blank_lines=1,
            file_extensions={".rs", ".toml"},
            exclude_patterns={
                "target/*",
                "*.lock",
                ".git/*",
                "processed_project/*",
                ".github/*",
                "docs/*"
            }
        )

        print("Starting Rust source code stripping...")
        print(f"Source directory: {config.source_dir}")
        print(f"Output directory: {config.output_dir}")

        # Create and run stripper
        stripper = RustSourceStripper(config)
        stats = stripper.process_project()

        # Report results
        if stats.files_failed > 0:
            print(f"\nProcessing completed with {stats.files_failed} failures")
            for error in stats.errors:
                print(f"Error: {error}")
            return 1

        print("\nProcessing completed successfully!")
        return 0

    except Exception as e:
        print(f"Script execution failed: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())