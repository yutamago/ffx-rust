/* cargo-shim: logs the wall time of cargo runs inside ffx-decomp checkouts.
 *
 * Installed as ~/.local/bin/cargo (tools/timing/install.sh), ahead of
 * ~/.cargo/bin in PATH. Outside an ffx-decomp checkout or worktree it execs
 * the real cargo at once, so nothing changes there. Inside one it runs cargo
 * as a child, waits, and appends one line to
 * ~/ffx-work/timing/<worktree>.tsv (docs/workflow.md, "Build and test
 * timing"). Only builds, checks, tests, clippy, run and xwin are timed;
 * other subcommands exec at once too. Exit status, stdio and signals pass through: the child shares
 * the shim's stdio and process group, signals sent to the shim with kill(2)
 * are forwarded, and a child killed by a signal kills the shim with the same
 * signal. When logging fails the command still runs.
 *
 * Environment:
 *   FFX_TIMING=0            never log (exec straight away)
 *   FFX_TIMING_DIR          log directory (default ~/ffx-work/timing)
 *   FFX_TIMING_REAL_CARGO   the real cargo (default $CARGO_HOME/bin/cargo,
 *                           else ~/.cargo/bin/cargo)
 *   FFX_TIMING_PARENT       set for the child; "cargo" there means a nested
 *                           cargo (cargo-xwin's inner build), not logged again
 *   FFX_TIMING_WORKTREE     log under this name instead of the checkout's
 *                           (mutants.py builds in a copy, ~/ffx-work/mutants/<wt>/tree)
 *
 * Build: cc -O2 -o cargo tools/timing/cargo-shim.c
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static char real_cargo[PATH_MAX];
static volatile sig_atomic_t child_pid = 0;
static volatile sig_atomic_t pending_sig = 0;

static void find_real_cargo(void) {
    const char *p = getenv("FFX_TIMING_REAL_CARGO");
    if (p && *p) {
        snprintf(real_cargo, sizeof real_cargo, "%s", p);
        return;
    }
    p = getenv("CARGO_HOME");
    if (p && *p) {
        snprintf(real_cargo, sizeof real_cargo, "%s/bin/cargo", p);
        if (access(real_cargo, X_OK) == 0)
            return;
    }
    p = getenv("HOME");
    snprintf(real_cargo, sizeof real_cargo, "%s/.cargo/bin/cargo", p ? p : "");
}

static void exec_real(char **argv) {
    execv(real_cargo, argv);
    fprintf(stderr, "cargo shim: cannot run %s: %s\n", real_cargo, strerror(errno));
    _exit(127);
}

/* Is dir (or a parent) an ffx-decomp checkout? Writes the root to out. */
static int find_root(const char *start, char *out, size_t n) {
    char dir[PATH_MAX], probe[PATH_MAX + 32];
    if (!realpath(start, dir))
        return 0;
    for (;;) {
        snprintf(probe, sizeof probe, "%s/symbols/ffx.sym", dir);
        if (access(probe, F_OK) == 0) {
            snprintf(probe, sizeof probe, "%s/tools/mutants.py", dir);
            if (access(probe, F_OK) == 0) {
                snprintf(out, n, "%s", dir);
                return 1;
            }
        }
        char *slash = strrchr(dir, '/');
        if (!slash || slash == dir)
            return 0;
        *slash = 0;
    }
}

/* The directory cargo works in: --manifest-path's or -C's, else the cwd. */
static int locate(int argc, char **argv, char *root, size_t n) {
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (strcmp(a, "--") == 0)
            break;
        const char *v = NULL;
        int is_manifest = 0;
        if (strcmp(a, "--manifest-path") == 0 && i + 1 < argc)
            v = argv[i + 1], is_manifest = 1;
        else if (strncmp(a, "--manifest-path=", 16) == 0)
            v = a + 16, is_manifest = 1;
        else if (strcmp(a, "-C") == 0 && i + 1 < argc)
            v = argv[i + 1];
        if (v) {
            char d[PATH_MAX];
            snprintf(d, sizeof d, "%s", v);
            if (is_manifest) {
                char *slash = strrchr(d, '/');
                if (slash)
                    *slash = 0;
                else
                    snprintf(d, sizeof d, ".");
            }
            return find_root(d[0] ? d : "/", root, n);
        }
    }
    return find_root(".", root, n);
}

/* Only builds and test runs are timed; `cargo metadata`, `--version`, `fmt`
 * and the like exec at once. */
static int timed(int argc, char **argv) {
    static const char *const subs[] = {"build", "b",     "check", "c",   "test",    "t",
                                       "clippy", "xwin", "run",   "r",   "bench",   "rustc",
                                       "doc",    "d",    "nextest", "llvm-cov", "miri", NULL};
    for (int i = 1; i < argc; i++) {
        const char *a = argv[i];
        if (a[0] == '+')
            continue;
        if (a[0] == '-') {
            if (!strcmp(a, "-C") || !strcmp(a, "-Z") || !strcmp(a, "--config") || !strcmp(a, "--color"))
                i++;
            continue;
        }
        for (int k = 0; subs[k]; k++)
            if (!strcmp(a, subs[k]))
                return 1;
        return 0;
    }
    return 0;
}

static void read_load(char *out, size_t n) {
    snprintf(out, n, "-");
    int fd = open("/proc/loadavg", O_RDONLY | O_CLOEXEC);
    if (fd < 0)
        return;
    char buf[64];
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0)
        return;
    buf[r] = 0;
    char *sp = strchr(buf, ' ');
    if (sp)
        *sp = 0;
    snprintf(out, n, "%s", buf);
}

static void forward(int sig, siginfo_t *info, void *ctx) {
    (void)ctx;
    /* Terminal signals (Ctrl-C) already reach the child: it shares our
     * process group. Forward only what was sent to the shim itself. */
    if (info && info->si_code == SI_KERNEL)
        return;
    if (child_pid > 0)
        kill(child_pid, sig);
    else
        pending_sig = sig;
}

static double now(clockid_t c) {
    struct timespec ts;
    clock_gettime(c, &ts);
    return ts.tv_sec + ts.tv_nsec / 1e9;
}

static void write_log(const char *root, int argc, char **argv, const char *iso, double wall,
                      int code, const char *load0, const char *load1, const char *parent) {
    const char *name = getenv("FFX_TIMING_WORKTREE"); /* set by a logging parent */
    if (!name || !*name || strchr(name, '/')) {
        name = strrchr(root, '/');
        name = name ? name + 1 : root;
    }
    char dir[PATH_MAX], path[2 * PATH_MAX + 16];
    const char *d = getenv("FFX_TIMING_DIR");
    if (d && *d)
        snprintf(dir, sizeof dir, "%s", d);
    else {
        const char *home = getenv("HOME");
        snprintf(dir, sizeof dir, "%s/ffx-work", home ? home : "");
        mkdir(dir, 0755);
        snprintf(dir, sizeof dir, "%s/ffx-work/timing", home ? home : "");
    }
    mkdir(dir, 0755);
    snprintf(path, sizeof path, "%s/%s.tsv", dir, name);

    char cmd[512];
    size_t len = (size_t)snprintf(cmd, sizeof cmd, "cargo");
    for (int i = 1; i < argc && len < sizeof cmd - 1; i++)
        len += (size_t)snprintf(cmd + len, sizeof cmd - len, " %s", argv[i]);
    if (len >= sizeof cmd)
        len = sizeof cmd - 1;
    for (size_t i = 0; i < len; i++)
        if (cmd[i] == '\t' || cmd[i] == '\n' || cmd[i] == '\r')
            cmd[i] = ' ';

    char line[2048];
    int n = snprintf(line, sizeof line, "%s\t%.3f\t%d\tcargo\t%s\t%s\t%s\t%s\t%s\n", iso, wall, code,
                     cmd, load0, load1, name, parent);
    if (n < 0)
        return;
    if ((size_t)n >= sizeof line) {
        n = sizeof line - 1;
        line[n - 1] = '\n';
    }
    int fd = open(path, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0644);
    if (fd < 0)
        return;
    ssize_t w = write(fd, line, (size_t)n); /* one write: O_APPEND keeps lines whole */
    (void)w;
    close(fd);
}

int main(int argc, char **argv) {
    find_real_cargo();
    const char *off = getenv("FFX_TIMING");
    const char *parent = getenv("FFX_TIMING_PARENT");
    if ((off && strcmp(off, "0") == 0) || (parent && strcmp(parent, "cargo") == 0))
        exec_real(argv);
    char root[PATH_MAX];
    if (!timed(argc, argv) || !locate(argc, argv, root, sizeof root))
        exec_real(argv);

    char parent_copy[128];
    snprintf(parent_copy, sizeof parent_copy, "%s", parent && *parent ? parent : "-");
    for (char *c = parent_copy; *c; c++)
        if (*c == '\t' || *c == '\n')
            *c = ' ';

    char load0[64], load1[64], iso[40];
    read_load(load0, sizeof load0);
    time_t t = time(NULL);
    struct tm tm;
    localtime_r(&t, &tm);
    strftime(iso, sizeof iso, "%Y-%m-%dT%H:%M:%S%z", &tm);
    size_t il = strlen(iso); /* +0200 -> +02:00 */
    if (il >= 5 && il + 1 < sizeof iso) {
        memmove(iso + il - 1, iso + il - 2, 3);
        iso[il - 2] = ':';
    }
    double t0 = now(CLOCK_MONOTONIC);
    setenv("FFX_TIMING_PARENT", "cargo", 1);

    struct sigaction sa, old[4];
    const int sigs[4] = {SIGINT, SIGTERM, SIGHUP, SIGQUIT};
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = forward;
    sa.sa_flags = SA_SIGINFO | SA_RESTART;
    sigemptyset(&sa.sa_mask);
    for (int i = 0; i < 4; i++)
        sigaction(sigs[i], &sa, &old[i]);

    pid_t pid = fork();
    if (pid < 0) { /* cannot fork: run cargo unlogged */
        for (int i = 0; i < 4; i++)
            sigaction(sigs[i], &old[i], NULL);
        exec_real(argv);
    }
    if (pid == 0) {
        /* exec resets caught signals to default; ignored ones stay ignored */
        for (int i = 0; i < 4; i++)
            sigaction(sigs[i], &old[i], NULL);
        exec_real(argv);
    }
    child_pid = pid;
    if (pending_sig)
        kill(pid, pending_sig);

    int status = 0;
    while (waitpid(pid, &status, 0) < 0 && errno == EINTR) {
    }
    double wall = now(CLOCK_MONOTONIC) - t0;
    int code = WIFEXITED(status) ? WEXITSTATUS(status) : 128 + WTERMSIG(status);
    read_load(load1, sizeof load1);
    write_log(root, argc, argv, iso, wall, code, load0, load1, parent_copy);

    if (WIFSIGNALED(status)) {
        int sig = WTERMSIG(status);
        struct rlimit no_core = {0, 0}; /* the child dumped core already */
        setrlimit(RLIMIT_CORE, &no_core);
        signal(sig, SIG_DFL);
        sigset_t set;
        sigemptyset(&set);
        sigaddset(&set, sig);
        sigprocmask(SIG_UNBLOCK, &set, NULL);
        raise(sig);
    }
    return code;
}
