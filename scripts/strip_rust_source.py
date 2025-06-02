#!/usr/bin/env python3
"""Aggressive Rust source code stripper for AI processing optimization.

This module provides functionality to aggressively strip Rust source code of ALL
unnecessary formatting, comments, and whitespace while preserving syntactic correctness.
Optimized for AI consumption where formatting is irrelevant.
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
    """Configuration for the aggressive Rust source code stripper."""

    source_dir: Path = Field(default=Path("."))
    output_dir: Path = Field(default=Path("ai_optimized_project"))
    preserve_cargo_toml: bool = Field(default=True)
    preserve_readme: bool = Field(default=False)
    file_extensions: Set[str] = Field(
        default_factory=lambda: {".rs", ".toml", ".md", ".txt"}
    )
    exclude_patterns: Set[str] = Field(
        default_factory=lambda: {"target/*", "*.lock", ".git/*", "ai_optimized_project/*"}
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


class AggressiveRustStripper:
    """Aggressive Rust source code stripper optimized for AI consumption."""

    def __init__(self, config: StripperConfig) -> None:
        """Initialize the aggressive Rust source stripper."""
        self.config = config
        self.stats = ProcessingStats()

        # Aggressive patterns for maximum compression
        self._all_comments_pattern = re.compile(r'//.*?$|/\*.*?\*/', re.MULTILINE | re.DOTALL)
        self._multiple_spaces_pattern = re.compile(r' {2,}')
        self._spaces_around_operators_pattern = re.compile(r'\s*([=+\-*/%<>!&|^~?:;,{}()\[\]])\s*')
        self._trailing_whitespace_pattern = re.compile(r'[ \t]+$', re.MULTILINE)
        self._blank_lines_pattern = re.compile(r'\n\s*\n+')
        self._leading_whitespace_pattern = re.compile(r'^\s+', re.MULTILINE)
        
        # Patterns for collapsing multi-line constructs
        self._multiline_fn_pattern = re.compile(r'fn\s+(\w+)\s*\([^)]*\)\s*->\s*[^{]*\{', re.MULTILINE | re.DOTALL)
        self._multiline_struct_pattern = re.compile(r'struct\s+(\w+)\s*[^{]*\{[^}]*\}', re.MULTILINE | re.DOTALL)
        self._multiline_impl_pattern = re.compile(r'impl\s*[^{]*\{', re.MULTILINE | re.DOTALL)
        
        # String literal protection patterns
        self._string_literals = []
        self._raw_string_pattern = re.compile(r'r#*".*?"#*', re.DOTALL)
        self._regular_string_pattern = re.compile(r'"(?:[^"\\]|\\.)*"', re.DOTALL)
        self._char_pattern = re.compile(r"'(?:[^'\\]|\\.)'")

    def process_project(self) -> ProcessingStats:
        """Process the entire Rust project with aggressive optimization."""
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
        if file_path.suffix not in self.config.file_extensions:
            return False

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
        """Process a single file with aggressive optimization."""
        try:
            original_content = self._read_file_content(file_path)
            original_size = len(original_content)
            original_lines = original_content.count('\n')

            if file_path.suffix == ".rs":
                processed_content = self._aggressively_strip_rust_file(original_content)
            else:
                processed_content = self._aggressively_strip_generic_file(original_content)

            processed_size = len(processed_content)
            processed_lines = processed_content.count('\n')

            self.stats.bytes_removed += original_size - processed_size
            self.stats.lines_removed += original_lines - processed_lines

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
            for encoding in ['utf-8', 'latin-1']:
                try:
                    return file_path.read_text(encoding=encoding)
                except UnicodeDecodeError:
                    continue

            raise FileProcessingError(f"Unable to decode file {file_path}")

        except OSError as e:
            raise FileProcessingError(f"Failed to read {file_path}: {e}") from e

    def _protect_string_literals(self, content: str) -> str:
        """Temporarily replace string literals with placeholders to protect them."""
        self._string_literals = []
        
        def replace_string(match):
            placeholder = f"__STRING_LITERAL_{len(self._string_literals)}__"
            self._string_literals.append(match.group(0))
            return placeholder
        
        # Protect raw strings first
        content = self._raw_string_pattern.sub(replace_string, content)
        # Then regular strings
        content = self._regular_string_pattern.sub(replace_string, content)
        # Then char literals
        content = self._char_pattern.sub(replace_string, content)
        
        return content

    def _restore_string_literals(self, content: str) -> str:
        """Restore protected string literals."""
        for i, literal in enumerate(self._string_literals):
            placeholder = f"__STRING_LITERAL_{i}__"
            content = content.replace(placeholder, literal)
        return content

    def _aggressively_strip_rust_file(self, content: str) -> str:
        """Aggressively strip a Rust source file for AI consumption."""
        # Protect string literals first
        content = self._protect_string_literals(content)
        
        # Remove ALL comments (not just doc comments)
        content = self._all_comments_pattern.sub('', content)
        
        # Remove all trailing whitespace
        content = self._trailing_whitespace_pattern.sub('', content)
        
        # Remove all leading whitespace (indentation)
        content = self._leading_whitespace_pattern.sub('', content)
        
        # Remove all blank lines
        content = self._blank_lines_pattern.sub('\n', content)
        
        # Minimize spaces around operators and punctuation
        # Be careful with certain operators that need spacing
        def minimize_operator_spacing(match):
            op = match.group(1)
            # Some operators need a space before them in certain contexts
            if op in ['-', '+'] and match.start() > 0:
                prev_char = content[match.start() - 1] if match.start() > 0 else ''
                if prev_char.isalnum() or prev_char in ')]}':
                    return f' {op}'
            # Most operators can be compressed
            if op in '{}()[],;':
                return op
            elif op in '=+-*/%<>!&|^~?:':
                return op
            return op
        
        content = self._spaces_around_operators_pattern.sub(minimize_operator_spacing, content)
        
        # Collapse multiple spaces into single spaces
        content = self._multiple_spaces_pattern.sub(' ', content)
        
        # Remove spaces after opening and before closing brackets/braces/parens
        content = re.sub(r'([\[\{\(])\s+', r'\1', content)
        content = re.sub(r'\s+([\]\}\)])', r'\1', content)
        
        # Collapse function signatures and other multi-line constructs
        content = self._collapse_multiline_constructs(content)
        
        # Final cleanup - remove any remaining multiple newlines
        content = re.sub(r'\n+', '\n', content)
        
        # Remove any remaining multiple spaces
        content = re.sub(r' +', ' ', content)
        
        # Restore string literals
        content = self._restore_string_literals(content)
        
        # Strip leading/trailing whitespace and ensure single trailing newline
        content = content.strip()
        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _collapse_multiline_constructs(self, content: str) -> str:
        """Collapse multi-line constructs into single lines where possible."""
        lines = content.split('\n')
        collapsed_lines = []
        i = 0
        
        while i < len(lines):
            line = lines[i].strip()
            
            # Skip empty lines
            if not line:
                i += 1
                continue
            
            # Try to collapse multi-line constructs
            if any(keyword in line for keyword in ['fn ', 'struct ', 'impl ', 'enum ', 'trait ']):
                # Look for opening brace on same or next few lines
                combined_line = line
                j = i + 1
                brace_count = line.count('{') - line.count('}')
                
                while j < len(lines) and j < i + 5:  # Look ahead max 5 lines
                    next_line = lines[j].strip()
                    if not next_line:
                        j += 1
                        continue
                    
                    combined_line += next_line
                    brace_count += next_line.count('{') - next_line.count('}')
                    
                    if '{' in next_line:
                        collapsed_lines.append(combined_line)
                        i = j + 1
                        break
                    j += 1
                else:
                    collapsed_lines.append(line)
                    i += 1
            else:
                collapsed_lines.append(line)
                i += 1
        
        return '\n'.join(collapsed_lines)

    def _aggressively_strip_generic_file(self, content: str) -> str:
        """Aggressively strip a generic file."""
        # For non-Rust files, still be aggressive but preserve basic structure
        content = self._trailing_whitespace_pattern.sub('', content)
        content = self._blank_lines_pattern.sub('\n', content)
        content = self._multiple_spaces_pattern.sub(' ', content)
        content = content.strip()

        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _write_processed_file(self, original_path: Path, content: str) -> None:
        """Write processed content to output file."""
        try:
            relative_path = original_path.relative_to(self.config.source_dir)
            output_path = self.config.output_dir / relative_path

            output_path.parent.mkdir(parents=True, exist_ok=True)
            output_path.write_text(content, encoding='utf-8')

        except OSError as e:
            raise FileProcessingError(f"Failed to write {output_path}: {e}") from e

    def _log_final_stats(self) -> None:
        """Log final processing statistics."""
        print(f"\nAggressive processing completed:")
        print(f"  Files processed: {self.stats.files_processed}")
        print(f"  Files skipped: {self.stats.files_skipped}")
        print(f"  Files failed: {self.stats.files_failed}")
        print(f"  Bytes removed: {self.stats.bytes_removed}")
        print(f"  Lines removed: {self.stats.lines_removed}")
        
        if self.stats.files_processed > 0:
            avg_reduction = self.stats.bytes_removed / self.stats.files_processed
            print(f"  Average bytes reduced per file: {avg_reduction:.1f}")
        
        if self.stats.errors:
            print(f"  Errors: {len(self.stats.errors)}")


def main() -> int:
    """Main entry point for aggressive Rust source stripping."""
    try:
        config = StripperConfig(
            source_dir=Path("."),
            output_dir=Path("processed_project"),
            preserve_cargo_toml=True,
            preserve_readme=False,
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

        print("Starting aggressive Rust source code stripping for AI optimization...")
        print("WARNING: This will remove ALL formatting, comments, and unnecessary whitespace!")
        print(f"Source directory: {config.source_dir}")
        print(f"Output directory: {config.output_dir}")

        stripper = AggressiveRustStripper(config)
        stats = stripper.process_project()

        if stats.files_failed > 0:
            print(f"\nProcessing completed with {stats.files_failed} failures")
            for error in stats.errors:
                print(f"Error: {error}")
            return 1

        print("\nAggressive processing completed successfully!")
        print("Code is now optimized for AI consumption with maximum compression.")
        return 0

    except Exception as e:
        print(f"Script execution failed: {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
