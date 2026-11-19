// The read fence's eyes in child processes (E11). read-fence.mjs sees what a node test process
// reads; this LD_PRELOAD library sees what every dynamically linked child (bash, git, python, sed,
// the Rust CLIs) opens, stats, lists or executes, and appends one JSON line per repository path to
// GATE_READ_FENCE_LOG. It judges nothing about reads: `node read-fence.mjs check` judges each row
// against the test file's declared reads afterwards. It does judge one thing at exec time, because
// only here can it be seen: an exec whose environment drops the fence, or whose program is static
// or set-id (LD_PRELOAD does not reach it), is logged as an unfenced child.
//
// Paths are resolved lexically against the cwd (or the *at directory descriptor), the same rule
// read-fence.mjs uses. Built by suite-run.mjs: cc -O2 -shared -fPIC -march=x86-64 -o <so> <this> -ldl
#define _GNU_SOURCE
#include <dirent.h>
#include <dlfcn.h>
#include <elf.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <spawn.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>

static __thread int busy;
static char root[PATH_MAX], gitdir[PATH_MAX], skip[PATH_MAX], logpath[PATH_MAX], file[512], self_so[PATH_MAX];
static int ready = -1;

static void init(void) {
  if (ready != -1) return;
  const char *r = getenv("GATE_READ_FENCE_ROOT"), *l = getenv("GATE_READ_FENCE_LOG");
  const char *g = getenv("GATE_READ_FENCE_GITDIR"), *f = getenv("GATE_READ_FENCE_FILE");
  const char *s = getenv("GATE_READ_FENCE_SO");
  if (!r || !l || !*r || !*l) { ready = 0; return; }
  snprintf(root, sizeof root, "%s", r);
  snprintf(logpath, sizeof logpath, "%s", l);
  snprintf(gitdir, sizeof gitdir, "%s", g ? g : "");
  (void)f;
  snprintf(skip, sizeof skip, "%s/genesis/orchestrator", r);
  snprintf(self_so, sizeof self_so, "%s", s ? s : "");
  ready = 1;
}

// Lexical normalization of an absolute path in place: drop "." and resolve "..".
static void normalize(char *p) {
  char out[PATH_MAX];
  size_t n = 0;
  const char *s = p;
  while (*s) {
    while (*s == '/') s++;
    if (!*s) break;
    const char *e = strchr(s, '/');
    size_t len = e ? (size_t)(e - s) : strlen(s);
    if (len == 1 && s[0] == '.') {
    } else if (len == 2 && s[0] == '.' && s[1] == '.') {
      while (n > 0 && out[n - 1] != '/') n--;
      if (n > 0) n--;
    } else if (n + 1 + len < sizeof out) {
      out[n++] = '/';
      memcpy(out + n, s, len);
      n += len;
    }
    s += len;
  }
  if (n == 0) out[n++] = '/';
  out[n] = 0;
  strcpy(p, out);
}

static int absolute(int dirfd, const char *path, char *out) {
  if (!path || !*path) return 0;
  if (path[0] == '/') {
    snprintf(out, PATH_MAX, "%s", path);
  } else {
    char base[PATH_MAX];
    if (dirfd == AT_FDCWD) {
      long n = syscall(SYS_getcwd, base, sizeof base);
      if (n <= 0) return 0;
    } else {
      char link[64];
      snprintf(link, sizeof link, "/proc/self/fd/%d", dirfd);
      ssize_t n = syscall(SYS_readlink, link, base, sizeof base - 1);
      if (n <= 0) return 0;
      base[n] = 0;
    }
    if (snprintf(out, PATH_MAX, "%s/%s", base, path) >= PATH_MAX) return 0;
  }
  normalize(out);
  return 1;
}

static int under(const char *dir, const char *p) {
  size_t n = strlen(dir);
  return n && strncmp(p, dir, n) == 0 && (p[n] == 0 || p[n] == '/');
}

// A node test process loads this library before read-fence.mjs names its file, and an exec into a
// hand-built environment loses the name: keep the last one this process saw.
static void refresh_file(void) {
  const char *f = getenv("GATE_READ_FENCE_FILE");
  if (f && *f) snprintf(file, sizeof file, "%s", f);
}

static void json_string(char *out, size_t cap, size_t *at, const char *s) {
  if (*at < cap) out[(*at)++] = '"';
  for (; *s && *at + 7 < cap; s++) {
    unsigned char c = (unsigned char)*s;
    if (c == '"' || c == '\\') { out[(*at)++] = '\\'; out[(*at)++] = c; }
    else if (c < 0x20) *at += snprintf(out + *at, cap - *at, "\\u%04x", c);
    else out[(*at)++] = c;
  }
  if (*at < cap) out[(*at)++] = '"';
}

static void emit(const char *kind, const char *op, const char *path, const char *reason) {
  char line[3 * PATH_MAX];
  size_t at = 0;
  refresh_file();
  at += snprintf(line + at, sizeof line - at, "{\"file\":");
  json_string(line, sizeof line, &at, file);
  at += snprintf(line + at, sizeof line - at, ",\"kind\":\"%s\",\"op\":\"%s\",\"path\":", kind, op);
  json_string(line, sizeof line, &at, path);
  if (reason) {
    at += snprintf(line + at, sizeof line - at, ",\"reason\":");
    json_string(line, sizeof line, &at, reason);
  }
  char exe[PATH_MAX];
  ssize_t n = syscall(SYS_readlink, "/proc/self/exe", exe, sizeof exe - 1);
  exe[n > 0 ? n : 0] = 0;
  at += snprintf(line + at, sizeof line - at, ",\"exe\":");
  json_string(line, sizeof line, &at, exe);
  at += snprintf(line + at, sizeof line - at, "}\n");
  if (at >= sizeof line) return;
  int fd = (int)syscall(SYS_openat, AT_FDCWD, logpath, O_WRONLY | O_APPEND | O_CREAT | O_CLOEXEC, 0600);
  if (fd < 0) return;
  syscall(SYS_write, fd, line, at);
  syscall(SYS_close, fd);
}

// Record one access to `path` (relative to dirfd) when it lies in the repository or its git dir.
static void seen(int dirfd, const char *path, const char *op) {
  if (busy) return;
  busy = 1;
  init();
  int saved = errno;
  refresh_file();
  char abs[PATH_MAX];
  if (ready == 1 && absolute(dirfd, path, abs)) {
    if (gitdir[0] && under(gitdir, abs)) {
      char rel[PATH_MAX + 16];
      snprintf(rel, sizeof rel, "<git-dir>%s", abs + strlen(gitdir));
      emit("child", op, rel, "repository object store read");
    } else if (under(root, abs) && !under(skip, abs) && !strstr(abs, "/node_modules/") && !strstr(abs, "/node_modules")) {
      emit("child", op, abs[strlen(root)] ? abs + strlen(root) + 1 : ".", NULL);
    }
  }
  errno = saved;
  busy = 0;
}

#define REAL(ret, name, ...) \
  static ret (*real_##name)(__VA_ARGS__); \
  if (!real_##name) real_##name = (ret (*)(__VA_ARGS__))dlsym(RTLD_NEXT, #name)

static int mode_arg(int flags, va_list ap) {
  return (flags & O_CREAT) || (flags & __O_TMPFILE) == __O_TMPFILE ? va_arg(ap, int) : 0;
}

int open(const char *p, int flags, ...) {
  va_list ap; va_start(ap, flags); int mode = mode_arg(flags, ap); va_end(ap);
  REAL(int, open, const char *, int, ...);
  seen(AT_FDCWD, p, (flags & O_DIRECTORY) ? "opendir" : "open");
  return real_open(p, flags, mode);
}
int open64(const char *p, int flags, ...) {
  va_list ap; va_start(ap, flags); int mode = mode_arg(flags, ap); va_end(ap);
  REAL(int, open64, const char *, int, ...);
  seen(AT_FDCWD, p, (flags & O_DIRECTORY) ? "opendir" : "open");
  return real_open64(p, flags, mode);
}
int openat(int d, const char *p, int flags, ...) {
  va_list ap; va_start(ap, flags); int mode = mode_arg(flags, ap); va_end(ap);
  REAL(int, openat, int, const char *, int, ...);
  seen(d, p, (flags & O_DIRECTORY) ? "opendir" : "open");
  return real_openat(d, p, flags, mode);
}
int openat64(int d, const char *p, int flags, ...) {
  va_list ap; va_start(ap, flags); int mode = mode_arg(flags, ap); va_end(ap);
  REAL(int, openat64, int, const char *, int, ...);
  seen(d, p, (flags & O_DIRECTORY) ? "opendir" : "open");
  return real_openat64(d, p, flags, mode);
}
int __open_2(const char *p, int flags) { REAL(int, __open_2, const char *, int); seen(AT_FDCWD, p, "open"); return real___open_2(p, flags); }
int __open64_2(const char *p, int flags) { REAL(int, __open64_2, const char *, int); seen(AT_FDCWD, p, "open"); return real___open64_2(p, flags); }
int __openat_2(int d, const char *p, int flags) { REAL(int, __openat_2, int, const char *, int); seen(d, p, "open"); return real___openat_2(d, p, flags); }
int __openat64_2(int d, const char *p, int flags) { REAL(int, __openat64_2, int, const char *, int); seen(d, p, "open"); return real___openat64_2(d, p, flags); }
FILE *fopen(const char *p, const char *m) { REAL(FILE *, fopen, const char *, const char *); seen(AT_FDCWD, p, "open"); return real_fopen(p, m); }
FILE *fopen64(const char *p, const char *m) { REAL(FILE *, fopen64, const char *, const char *); seen(AT_FDCWD, p, "open"); return real_fopen64(p, m); }
FILE *freopen(const char *p, const char *m, FILE *f) { REAL(FILE *, freopen, const char *, const char *, FILE *); seen(AT_FDCWD, p, "open"); return real_freopen(p, m, f); }
int stat(const char *p, struct stat *b) { REAL(int, stat, const char *, struct stat *); seen(AT_FDCWD, p, "stat"); return real_stat(p, b); }
int lstat(const char *p, struct stat *b) { REAL(int, lstat, const char *, struct stat *); seen(AT_FDCWD, p, "stat"); return real_lstat(p, b); }
int stat64(const char *p, struct stat64 *b) { REAL(int, stat64, const char *, struct stat64 *); seen(AT_FDCWD, p, "stat"); return real_stat64(p, b); }
int lstat64(const char *p, struct stat64 *b) { REAL(int, lstat64, const char *, struct stat64 *); seen(AT_FDCWD, p, "stat"); return real_lstat64(p, b); }
int fstatat(int d, const char *p, struct stat *b, int f) { REAL(int, fstatat, int, const char *, struct stat *, int); seen(d, p, "stat"); return real_fstatat(d, p, b, f); }
int fstatat64(int d, const char *p, struct stat64 *b, int f) { REAL(int, fstatat64, int, const char *, struct stat64 *, int); seen(d, p, "stat"); return real_fstatat64(d, p, b, f); }
int statx(int d, const char *p, int f, unsigned m, struct statx *b) { REAL(int, statx, int, const char *, int, unsigned, struct statx *); seen(d, p, "stat"); return real_statx(d, p, f, m, b); }
int __xstat(int v, const char *p, struct stat *b) { REAL(int, __xstat, int, const char *, struct stat *); seen(AT_FDCWD, p, "stat"); return real___xstat(v, p, b); }
int __lxstat(int v, const char *p, struct stat *b) { REAL(int, __lxstat, int, const char *, struct stat *); seen(AT_FDCWD, p, "stat"); return real___lxstat(v, p, b); }
int __xstat64(int v, const char *p, struct stat64 *b) { REAL(int, __xstat64, int, const char *, struct stat64 *); seen(AT_FDCWD, p, "stat"); return real___xstat64(v, p, b); }
int __lxstat64(int v, const char *p, struct stat64 *b) { REAL(int, __lxstat64, int, const char *, struct stat64 *); seen(AT_FDCWD, p, "stat"); return real___lxstat64(v, p, b); }
int __fxstatat(int v, int d, const char *p, struct stat *b, int f) { REAL(int, __fxstatat, int, int, const char *, struct stat *, int); seen(d, p, "stat"); return real___fxstatat(v, d, p, b, f); }
int access(const char *p, int m) { REAL(int, access, const char *, int); seen(AT_FDCWD, p, "stat"); return real_access(p, m); }
int faccessat(int d, const char *p, int m, int f) { REAL(int, faccessat, int, const char *, int, int); seen(d, p, "stat"); return real_faccessat(d, p, m, f); }
int euidaccess(const char *p, int m) { REAL(int, euidaccess, const char *, int); seen(AT_FDCWD, p, "stat"); return real_euidaccess(p, m); }
int eaccess(const char *p, int m) { REAL(int, eaccess, const char *, int); seen(AT_FDCWD, p, "stat"); return real_eaccess(p, m); }
ssize_t readlink(const char *p, char *b, size_t n) { REAL(ssize_t, readlink, const char *, char *, size_t); seen(AT_FDCWD, p, "stat"); return real_readlink(p, b, n); }
ssize_t readlinkat(int d, const char *p, char *b, size_t n) { REAL(ssize_t, readlinkat, int, const char *, char *, size_t); seen(d, p, "stat"); return real_readlinkat(d, p, b, n); }
DIR *opendir(const char *p) { REAL(DIR *, opendir, const char *); seen(AT_FDCWD, p, "opendir"); return real_opendir(p); }
int chdir(const char *p) { REAL(int, chdir, const char *); seen(AT_FDCWD, p, "stat"); return real_chdir(p); }

// ---- exec: the program is a read, and the fence must reach the new image ----

static int env_has(char *const envp[], const char *key, const char *value) {
  size_t k = strlen(key);
  for (char *const *e = envp; e && *e; e++) {
    if (strncmp(*e, key, k) == 0 && (*e)[k] == '=') return value ? strstr(*e + k + 1, value) != NULL : 1;
  }
  return 0;
}

// 1 when the ELF at `path` has a program interpreter (dynamically linked), 0 static, -1 not ELF.
static int dynamic_elf(const char *path) {
  int fd = (int)syscall(SYS_openat, AT_FDCWD, path, O_RDONLY | O_CLOEXEC);
  if (fd < 0) return -1;
  Elf64_Ehdr h;
  int result = -1;
  if (syscall(SYS_pread64, fd, &h, sizeof h, 0) == sizeof h && memcmp(h.e_ident, ELFMAG, SELFMAG) == 0
      && h.e_ident[EI_CLASS] == ELFCLASS64) {
    result = 0;
    for (int i = 0; i < h.e_phnum; i++) {
      Elf64_Phdr ph;
      if (syscall(SYS_pread64, fd, &ph, sizeof ph, (off_t)(h.e_phoff + (Elf64_Off)i * h.e_phentsize)) != sizeof ph) break;
      if (ph.p_type == PT_INTERP) { result = 1; break; }
    }
  }
  syscall(SYS_close, fd);
  return result;
}

// Resolve a program name the way execvp does, for judging only.
static int resolve_program(const char *name, char *out) {
  if (strchr(name, '/')) return absolute(AT_FDCWD, name, out);
  const char *path = getenv("PATH");
  if (!path) path = "/usr/bin:/bin";
  while (*path) {
    const char *e = strchr(path, ':');
    size_t len = e ? (size_t)(e - path) : strlen(path);
    if (snprintf(out, PATH_MAX, "%.*s/%s", (int)len, path, name) < PATH_MAX && syscall(SYS_faccessat, AT_FDCWD, out, X_OK) == 0) return 1;
    path += len + (e ? 1 : 0);
  }
  return 0;
}

static int names_repo(const char *value) {
  const char *dirs[2] = { root, gitdir };
  for (int d = 0; d < 2; d++) {
    size_t n = strlen(dirs[d]);
    if (!n) continue;
    for (const char *at = strstr(value, dirs[d]); at; at = strstr(at + 1, dirs[d])) {
      char next = at[n];
      if (next == 0 || next == '/' || !(next == '_' || next == '.' || next == '-' || (next >= '0' && next <= '9')
          || (next >= 'a' && next <= 'z') || (next >= 'A' && next <= 'Z'))) return 1;
    }
  }
  return 0;
}

// An exec the library will not follow is acceptable when nothing ties it to the repository: its
// cwd is outside it, and no argument, and no environment value but where-the-shell-was, names it.
static int fixture_bound(const char *program, char *const argv[], char *const envp[]) {
  char cwd[PATH_MAX];
  if (syscall(SYS_getcwd, cwd, sizeof cwd) <= 0) return 0;
  if (under(root, cwd) || (gitdir[0] && under(gitdir, cwd))) return 0;
  if (names_repo(program)) return 0;
  for (char *const *a = argv; a && *a; a++) if (names_repo(*a)) return 0;
  for (char *const *e = envp; e && *e; e++) {
    if (!strncmp(*e, "PWD=", 4) || !strncmp(*e, "OLDPWD=", 7) || !strncmp(*e, "INIT_CWD=", 9)
        || !strncmp(*e, "GATE_READ_FENCE_", 16)) continue; // where the shell was; the fence's own names
    const char *eq = strchr(*e, '=');
    if (eq && names_repo(eq + 1)) return 0;
  }
  return 1;
}

static void judge_exec(const char *name, char *const argv[], char *const envp[]) {
  if (busy) return;
  init();
  if (ready != 1) return;
  char program[PATH_MAX];
  if (!resolve_program(name, program)) return;
  seen(AT_FDCWD, program, "exec");
  busy = 1;
  const char *reason = NULL;
  struct stat st;
  int followed = env_has(envp, "LD_PRELOAD", self_so[0] ? self_so : "read-fence-preload")
    && env_has(envp, "GATE_READ_FENCE_LOG", logpath) && env_has(envp, "GATE_READ_FENCE_ROOT", root);
  if (!followed) reason = "child cleared the read fence";
  else if (syscall(SYS_stat, program, &st) == 0 && (st.st_mode & (S_ISUID | S_ISGID))) reason = "set-id program, unseen by the read fence";
  else if (dynamic_elf(program) == 0) reason = "static program, unseen by the read fence";
  if (reason && fixture_bound(program, argv, envp)) reason = NULL;
  if (reason) emit("spawn", "exec", program, reason);
  busy = 0;
}

extern char **environ;
int execve(const char *p, char *const a[], char *const e[]) { REAL(int, execve, const char *, char *const[], char *const[]); judge_exec(p, a, e); return real_execve(p, a, e); }
int execv(const char *p, char *const a[]) { REAL(int, execv, const char *, char *const[]); judge_exec(p, a, environ); return real_execv(p, a); }
int execvp(const char *p, char *const a[]) { REAL(int, execvp, const char *, char *const[]); judge_exec(p, a, environ); return real_execvp(p, a); }
int execvpe(const char *p, char *const a[], char *const e[]) { REAL(int, execvpe, const char *, char *const[], char *const[]); judge_exec(p, a, e); return real_execvpe(p, a, e); }
int posix_spawn(pid_t *pid, const char *p, const posix_spawn_file_actions_t *fa, const posix_spawnattr_t *at, char *const a[], char *const e[]) {
  REAL(int, posix_spawn, pid_t *, const char *, const posix_spawn_file_actions_t *, const posix_spawnattr_t *, char *const[], char *const[]);
  judge_exec(p, a, e);
  return real_posix_spawn(pid, p, fa, at, a, e);
}
int posix_spawnp(pid_t *pid, const char *p, const posix_spawn_file_actions_t *fa, const posix_spawnattr_t *at, char *const a[], char *const e[]) {
  REAL(int, posix_spawnp, pid_t *, const char *, const posix_spawn_file_actions_t *, const posix_spawnattr_t *, char *const[], char *const[]);
  judge_exec(p, a, e);
  return real_posix_spawnp(pid, p, fa, at, a, e);
}
