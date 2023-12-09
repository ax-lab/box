package util

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"sync"
	"syscall"
)

func ExecInDir(prefix, dir string, callback func() bool) bool {
	noError := func(op string, err error) bool {
		if err != nil {
			fmt.Fprintf(os.Stderr, "%s ExecInDir: %s: %v\n", prefix, op, err)
		}
		return err == nil
	}

	var success bool
	cwd, err := os.Getwd()
	if noError("getting current dir", err) {
		err = os.Chdir(dir)
		if noError("changing dir", err) {
			success = callback()
		}
	}

	err = os.Chdir(cwd)
	return noError("restoring working dir", err) && success
}

// Spawn a new process "replaces" the current process by the given one.
//
// The new process shares the same environment and standard output streams
// as the current process.
//
// After the spawned process exits, the current process will exit with the
// same exit code.
func Spawn(name string, args ...string) error {
	files := make([]*os.File, 3)
	files[syscall.Stdin] = os.Stdin
	files[syscall.Stdout] = os.Stdout
	files[syscall.Stderr] = os.Stderr

	proc, err := os.StartProcess(name, args, &os.ProcAttr{
		Dir:   ".",
		Env:   os.Environ(),
		Files: files,
	})

	if err != nil {
		return fmt.Errorf("spawn: %v", err)
	}

	state, err := proc.Wait()
	if err != nil {
		return fmt.Errorf("spawn: wait failed: %v", err)
	}

	os.Exit(state.ExitCode())
	panic("unreachable")
}

// Run a new process handling errors and stderr output.
func Run(prefix, name string, args ...string) bool {
	var lf = []byte("\n")
	cmd := exec.Command(name, args...)

	stderr, err := cmd.StderrPipe()
	if err != nil {
		fmt.Fprintf(os.Stderr, "\n%s io error: %v\n\n", prefix, err)
		return false
	}

	if err = cmd.Start(); err != nil {
		fmt.Fprintf(os.Stderr, "\n%s start error: %v\n\n", prefix, err)
		return false
	}

	errors, eol := false, true
	buffer := make([]byte, 4096)
	for {
		n, err := stderr.Read(buffer)
		if n > 0 {
			if !errors {
				os.Stderr.Write(lf)
			}

			errors = true
			lines := bytes.Split(buffer[:n], lf)
			for i, line := range lines {
				if i > 0 {
					os.Stderr.Write(lf)
					eol = true
				}
				if eol && (len(line) > 0 || i < len(lines)-1) {
					os.Stderr.Write([]byte(prefix))
					os.Stderr.Write([]byte(" | "))
					eol = false
				}

				os.Stderr.Write(line)
			}
		}
		if err == io.EOF {
			break
		}
	}

	if err = cmd.Wait(); err != nil {
		fmt.Fprintf(os.Stderr, "\n%s command error: %v\n\n", prefix, err)
		return false
	}

	return true
}

// Exec a process using a callback to process output.
func Exec(name string, args []string, callback func(output string, isError bool)) (int, error) {
	cmd := exec.Command(name, args...)

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return -1, err
	}
	stderr, err := cmd.StderrPipe()
	if err != nil {
		return -1, err
	}

	if err = cmd.Start(); err != nil {
		return -1, err
	}

	wg := sync.WaitGroup{}

	consume := func(output io.Reader, isError bool) {
		defer wg.Done()
		buffer := make([]byte, 4096)
		for {
			n, err := output.Read(buffer)
			if n > 0 {
				callback(string(buffer[:n]), isError)
			}
			if err == io.EOF {
				break
			}
		}
	}

	wg.Add(2)
	go consume(stdout, false)
	go consume(stderr, true)

	status, err := cmd.Process.Wait()
	wg.Wait()
	return status.ExitCode(), err
}
