package main

import (
	"compress/gzip"
	"context"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"time"
)

// version is auto-updated by scripts/semantic-release-prepare.sh
const Version = "0.9.0"

func main() {
	// Initialize structured logger
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	if err := run(); err != nil {
		slog.Error("Application failed", "error", err)
		os.Exit(1)
	}
}

func run() error {
	workDir, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("getting working directory: %w", err)
	}

	targetDir := filepath.Join(workDir, "lib")
	if len(os.Args) > 1 {
		targetDir = os.Args[1]
	}

	targetFile, err := filepath.Abs(filepath.Join(targetDir, "libindexer_c_bindings.a"))
	if err != nil {
		return fmt.Errorf("getting target location: %w", err)
	}

	if _, err := os.Stat(targetFile); err == nil {
		slog.Info("File already exists, skipping download", "path", targetFile)
		return nil
	}

	// when the repository is public, we can switch to
	// https://gitlab.com/gitlab-org/rust/knowledge-graph/-/releases URL
	// for now REST API is used to authenticate with GITLAB_TOKEN
	projectId := "69095239" // https://gitlab.com/gitlab-org/rust/knowledge-graph
	platform := runtime.GOOS + "-" + runtime.GOARCH
	url := fmt.Sprintf("https://gitlab.com/api/v4/projects/%s/packages/generic/release/%s/libindexer_c_bindings-%s.a.gz", projectId, Version, platform)

	slog.Info("Starting download",
		"version", Version,
		"platform", platform,
		"target", targetFile)

	if err := downloadAndExtract(url, targetFile); err != nil {
		return fmt.Errorf("downloading and extracting library: %w", err)
	}

	slog.Info("Successfully downloaded and extracted library", "path", targetFile)
	return nil
}

func downloadAndExtract(url string, targetFile string) error {
	tmpFile, err := os.CreateTemp("", "libindexer_c_bindings-*.gz")
	if err != nil {
		return fmt.Errorf("creating temporary file: %w", err)
	}
	defer func() {
		tmpFile.Close()
		if err := os.Remove(tmpFile.Name()); err != nil {
			slog.Warn("Failed to remove temporary file", "file", tmpFile.Name(), "error", err)
		}
	}()

	// Create HTTP client with timeout
	client := &http.Client{
		Timeout: 5 * time.Minute,
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return fmt.Errorf("creating request: %w", err)
	}

	// Use GITLAB_TOKEN instead of TOKEN
	if token := os.Getenv("GITLAB_TOKEN"); token != "" {
		req.Header.Set("PRIVATE-TOKEN", token)
		slog.Debug("Using GitLab token for authentication")
	} else {
		slog.Warn("No GITLAB_TOKEN found, proceeding without authentication")
	}

	slog.Info("Downloading file", "url", url)
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("downloading file: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, resp.Status)
	}

	// Log content length if available
	if resp.ContentLength > 0 {
		slog.Info("Download started", "size_bytes", resp.ContentLength)
	}

	if _, err = io.Copy(tmpFile, resp.Body); err != nil {
		return fmt.Errorf("writing to temporary file: %w", err)
	}

	if _, err = tmpFile.Seek(0, io.SeekStart); err != nil {
		return fmt.Errorf("seeking temporary file: %w", err)
	}

	// Ensure target directory exists
	targetDir := filepath.Dir(targetFile)
	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("creating target directory %s: %w", targetDir, err)
	}

	out, err := os.Create(targetFile)
	if err != nil {
		return fmt.Errorf("creating target file %s: %w", targetFile, err)
	}
	defer out.Close()

	gzr, err := gzip.NewReader(tmpFile)
	if err != nil {
		return fmt.Errorf("creating gzip reader: %w", err)
	}
	defer gzr.Close()

	slog.Info("Extracting file", "target", targetFile)
	if _, err = io.Copy(out, gzr); err != nil {
		return fmt.Errorf("extracting content: %w", err)
	}

	return nil
}
