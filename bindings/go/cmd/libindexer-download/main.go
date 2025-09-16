package main

import (
	"archive/tar"
	"compress/gzip"
	"context"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"log/slog"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"runtime"
	"time"
)

// version is auto-updated by scripts/semantic-release-prepare.sh
const Version = "0.14.0"

func main() {
	// Initialize structured logger
	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	ctx := context.Background()

	if err := run(ctx); err != nil {
		slog.Error("Application failed", "error", err)
		os.Exit(1)
	}
}

func run(ctx context.Context) error {
	workDir, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("getting working directory: %w", err)
	}

	targetDir := filepath.Join(workDir, "libindexer")
	if len(os.Args) > 1 {
		targetDir = os.Args[1]
	}

	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("mkdirall: %w", err)
	}

	root, err := os.OpenRoot(targetDir)
	if err != nil {
		return fmt.Errorf("open root: %w", err)
	}

	if _, err := root.Stat(path.Join("lib", "libindexer_c_bindings.a")); err == nil {
		slog.Info("File already exists, skipping download")
		return nil
	}

	projectId := "69095239" // https://gitlab.com/gitlab-org/rust/knowledge-graph
	platform := runtime.GOOS + "-" + runtime.GOARCH
	url := fmt.Sprintf("https://gitlab.com/api/v4/projects/%s/packages/generic/release/%s/libindexer_c_bindings-%s.tar.gz", projectId, Version, platform)

	slog.Info("Starting download",
		"version", Version,
		"platform", platform,
		"target", targetDir)

	if err := downloadAndExtract(ctx, url, root); err != nil {
		return fmt.Errorf("downloading and extracting library: %w", err)
	}

	slog.Info("Successfully downloaded and extracted library", "path", targetDir)
	return nil
}

func downloadAndExtract(ctx context.Context, url string, root *os.Root) (retErr error) {
	ctx, cancel := context.WithTimeout(ctx, 10*time.Minute)
	defer cancel()

	client := &http.Client{
		Timeout: 5 * time.Minute,
	}

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return fmt.Errorf("creating request: %w", err)
	}

	slog.Info("Downloading file", "url", url)
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("downloading file: %w", err)
	}
	defer func() {
		err := resp.Body.Close()
		if retErr == nil && err != nil {
			retErr = fmt.Errorf("closing file: %w", err)
		}
	}()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("HTTP %d: %s", resp.StatusCode, resp.Status)
	}

	// Log content length if available
	if resp.ContentLength > 0 {
		slog.Info("Download started", "size_bytes", resp.ContentLength)
	}

	gzr, err := gzip.NewReader(resp.Body)
	if err != nil {
		return fmt.Errorf("creating gzip reader: %w", err)
	}
	defer func() {
		err := gzr.Close()
		if err != nil && retErr == nil {
			retErr = fmt.Errorf("close gzip: %w", err)
		}
	}()

	tarReader := tar.NewReader(gzr)
	slog.Info("Extracting file")
	for {
		header, err := tarReader.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return fmt.Errorf("reading tar: %w", err)
		}

		if err := extractTarHeader(tarReader, header, root); err != nil {
			return err
		}
	}
	return nil
}

func extractTarHeader(tarReader *tar.Reader, header *tar.Header, root *os.Root) (retErr error) {
	switch header.Typeflag {
	case tar.TypeDir:
		err := root.Mkdir(header.Name, 0755)
		if err != nil && !errors.Is(err, fs.ErrExist) {
			return fmt.Errorf("mkdirall: %q: %w", header.Name, err)
		}
	case tar.TypeReg:
		outFile, err := root.Create(header.Name)
		if err != nil {
			return fmt.Errorf("create file: %w", err)
		}
		defer func() {
			err := outFile.Close()
			if retErr == nil && err != nil {
				retErr = fmt.Errorf("close file: %w", err)
			}
		}()

		if _, err := io.Copy(outFile, tarReader); err != nil {
			return fmt.Errorf("copy: %w", err)
		}
	default:
		return fmt.Errorf("unknown type: %c in %s", header.Typeflag, header.Name)
	}

	return nil
}
