#!/usr/bin/env python3
"""Rust source code stripper for AI processing optimization.

This module provides functionality to strip Rust source code of unnecessary
elements like documentation comments, excessive whitespace, and formatting
while preserving all functional code.
"""

from __future__ import annotations

import logging
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple, Union

import structlog
from pydantic import BaseModel, Field, validator


class RustStripperError(Exception):
    """Base exception for Rust source code stripping operations."""
    pass


class FileProcessingError(RustStripperError):
    """Exception raised when file processing fails."""
    pass


class ValidationError(RustStripperError):
    """Exception raised when input validation fails."""
    pass


class StripperConfig(BaseModel):
    """Configuration for the Rust source code stripper.

    Attributes:
        source_dir: Path to the source directory to process
        output_dir: Path to the output directory for processed files
        preserve_cargo_toml: Whether to preserve Cargo.toml files
        preserve_readme: Whether to preserve README files
        max_consecutive_blank_lines: Maximum consecutive blank lines to keep
        file_extensions: Set of file extensions to process
        exclude_patterns: Set of glob patterns to exclude from processing
    """

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
    """Statistics for file processing operations.

    Attributes:
        files_processed: Number of files successfully processed
        files_skipped: Number of files skipped
        files_failed: Number of files that failed processing
        bytes_removed: Total bytes removed from source files
        lines_removed: Total lines removed from source files
        errors: List of processing errors encountered
    """

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
    """Main class for stripping Rust source code files.

    This class handles the processing of Rust source files to remove
    documentation comments, excessive whitespace, and other non-functional
    elements while preserving code functionality.
    """

    def __init__(self, config: StripperConfig) -> None:
        """Initialize the Rust source stripper.

        Args:
            config: Configuration object for the stripper

        Raises:
            ValidationError: If configuration validation fails
        """
        self.config = config
        self.stats = ProcessingStats()
        self.logger = structlog.get_logger(__name__)

        # Compile regex patterns for performance
        self._doc_comment_pattern = re.compile(
            r'^\s*///.*$|^\s*/\*\*.*?\*/\s*$',
            re.MULTILINE | re.DOTALL
        )
        self._block_comment_pattern = re.compile(
            r'/\*\*.*?\*/',
            re.DOTALL
        )
        self._excessive_whitespace_pattern = re.compile(r'\n\s*\n\s*\n+')
        self._trailing_whitespace_pattern = re.compile(r'[ \t]+$', re.MULTILINE)

    def process_project(self) -> ProcessingStats:
        """Process the entire Rust project.

        Returns:
            ProcessingStats object containing processing results

        Raises:
            FileProcessingError: If critical processing operations fail
        """
        try:
            self._create_output_directory()
            self._process_all_files()
            self._log_final_stats()
            return self.stats

        except Exception as e:
            error_msg = f"Failed to process project: {e}"
            self.logger.error(error_msg)
            raise FileProcessingError(error_msg) from e

    def _create_output_directory(self) -> None:
        """Create the output directory structure."""
        try:
            self.config.output_dir.mkdir(parents=True, exist_ok=True)
            self.logger.info("Created output directory", path=str(self.config.output_dir))

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
                self.logger.error(error_msg)

    def _get_files_to_process(self) -> List[Path]:
        """Get list of files to process based on configuration.

        Returns:
            List of Path objects for files to process
        """
        files_to_process: List[Path] = []

        for file_path in self.config.source_dir.rglob("*"):
            if (
                file_path.is_file()
                and self._should_process_file(file_path)
                and not self._is_excluded(file_path)
            ):
                files_to_process.append(file_path)

        self.logger.info("Found files to process", count=len(files_to_process))
        return files_to_process

    def _should_process_file(self, file_path: Path) -> bool:
        """Determine if a file should be processed.

        Args:
            file_path: Path to the file to check

        Returns:
            True if the file should be processed, False otherwise
        """
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
        """Check if a file matches any exclusion patterns.

        Args:
            file_path: Path to check against exclusion patterns

        Returns:
            True if the file should be excluded, False otherwise
        """
        relative_path = file_path.relative_to(self.config.source_dir)

        for pattern in self.config.exclude_patterns:
            if relative_path.match(pattern):
                return True

        return False

    def _process_single_file(self, file_path: Path) -> None:
        """Process a single file.

        Args:
            file_path: Path to the file to process

        Raises:
            FileProcessingError: If file processing fails
        """
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
            self.logger.debug(
                "Processed file",
                file=str(file_path),
                original_size=original_size,
                processed_size=processed_size,
                reduction_percent=round((1 - processed_size / original_size) * 100, 2)
            )

        except Exception as e:
            raise FileProcessingError(f"Failed to process {file_path}: {e}") from e

    def _read_file_content(self, file_path: Path) -> str:
        """Read content from a file with proper encoding handling.

        Args:
            file_path: Path to the file to read

        Returns:
            File content as string

        Raises:
            FileProcessingError: If file reading fails
        """
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
        """Strip a Rust source file of unnecessary elements.

        Args:
            content: Original file content

        Returns:
            Stripped content
        """
        # Remove documentation comments (/// and /** */)
        content = self._doc_comment_pattern.sub('', content)
        content = self._block_comment_pattern.sub('', content)

        # Remove trailing whitespace
        content = self._trailing_whitespace_pattern.sub('', content)

        # Normalize blank lines
        content = self._normalize_blank_lines(content)

        # Remove leading/trailing whitespace from the entire file
        content = content.strip()

        # Ensure file ends with single newline
        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _strip_generic_file(self, content: str) -> str:
        """Strip a generic file of unnecessary whitespace.

        Args:
            content: Original file content

        Returns:
            Stripped content
        """
        # For non-Rust files, just normalize whitespace
        content = self._trailing_whitespace_pattern.sub('', content)
        content = self._normalize_blank_lines(content)
        content = content.strip()

        if content and not content.endswith('\n'):
            content += '\n'

        return content

    def _normalize_blank_lines(self, content: str) -> str:
        """Normalize excessive blank lines.

        Args:
            content: Content to normalize

        Returns:
            Content with normalized blank lines
        """
        max_lines = self.config.max_consecutive_blank_lines
        replacement = '\n' * (max_lines + 1)  # +1 for the content line

        return self._excessive_whitespace_pattern.sub(replacement, content)

    def _write_processed_file(self, original_path: Path, content: str) -> None:
        """Write processed content to output file.

        Args:
            original_path: Original file path
            content: Processed content to write

        Raises:
            FileProcessingError: If file writing fails
        """
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
        self.logger.info(
            "Processing completed",
            files_processed=self.stats.files_processed,
            files_skipped=self.stats.files_skipped,
            files_failed=self.stats.files_failed,
            bytes_removed=self.stats.bytes_removed,
            lines_removed=self.stats.lines_removed,
            total_errors=len(self.stats.errors)
        )


def setup_logging() -> None:
    """Set up structured logging configuration."""
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s"
    )

    structlog.configure(
        processors=[
            structlog.stdlib.filter_by_level,
            structlog.stdlib.add_logger_name,
            structlog.stdlib.add_log_level,
            structlog.stdlib.PositionalArgumentsFormatter(),
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
            structlog.processors.UnicodeDecoder(),
            structlog.processors.JSONRenderer()
        ],
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        wrapper_class=structlog.stdlib.BoundLogger,
        cache_logger_on_first_use=True,
    )


def main() -> int:
    """Main entry point for the script.

    Returns:
        Exit code (0 for success, 1 for failure)
    """
    setup_logging()
    logger = structlog.get_logger(__name__)

    try:
        # Load configuration
        config = StripperConfig()

        logger.info(
            "Starting Rust source code stripping",
            source_dir=str(config.source_dir),
            output_dir=str(config.output_dir)
        )

        # Create and run stripper
        stripper = RustSourceStripper(config)
        stats = stripper.process_project()

        # Report results
        if stats.files_failed > 0:
            logger.warning(f"Processing completed with {stats.files_failed} failures")
            for error in stats.errors:
                logger.error("Processing error", error=error)
            return 1

        logger.info("Processing completed successfully")
        return 0

    except Exception as e:
        logger.error("Script execution failed", error=str(e))
        return 1


if __name__ == "__main__":
    sys.exit(main())