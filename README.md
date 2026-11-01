# maiden-run

A command-line tool for automatically running norns scripts when files change. Perfect for rapid development workflows with norns.

## Installation

Install via cargo:

```bash
git clone https://github.com/ngwese/maiden-run.git
cd maiden-run
cargo install --path .
```

## Usage

### Basic Usage

The simplest way to run a script by name (watch mode is ON by default):

```bash
maiden-run awake
```

This will:
1. Load `code/awake/awake.lua` on your norns
2. Watch the current directory for changes
3. Automatically reload the script when files change

### Script Name Formats

**Simple names** are automatically expanded to the standard norns directory structure:

```bash
maiden-run awake          # loads code/awake/awake.lua
maiden-run awake.lua      # loads code/awake/awake.lua
maiden-run mlr            # loads code/mlr/mlr.lua
```

**Paths** are passed through as-is:

```bash
maiden-run code/myproject/main.lua       # loads code/myproject/main.lua
maiden-run dust/code/awake/awake.lua     # loads dust/code/awake/awake.lua
```

### Watch Mode

Watch mode is **enabled by default** and watches the current directory (`.`).

**Watch specific directories:**

```bash
maiden-run awake src lib          # watches src/ and lib/ directories
```

**Disable watch mode** (run once and exit):

```bash
maiden-run --no-watch awake
```

### Endpoint Configuration

**Using norns.local** (default):

```bash
maiden-run awake                  # connects to ws://norns.local:5555/
```

**Using localhost:**

```bash
maiden-run --endpoint ws://localhost:5555/ awake
```

Or use the short flag:

```bash
maiden-run -e ws://localhost:5555/ awake
```

### Typical Workflow

The most common usage when developing:

```bash
cd /path/to/your/norns/project
maiden-run myproject
```

This watches the current directory and reloads `code/myproject/myproject.lua` whenever files change.

## Options

- `-e, --endpoint <URL>` - Matron websocket URL (default: `ws://norns.local:5555/`)
- `--no-watch` - Disable watch mode (run once and exit)
- `<script>` - Script name or path to run (required)
- `<dirs>...` - Directories to watch (default: `.` when watch mode is enabled)
- `-h, --help` - Print help
- `-V, --version` - Print version

## Requirements

- A norns device accessible on your network
- The matron websocket must be accessible (default port 5555)
- For localhost connections: SSH tunnel or local norns development setup

## Troubleshooting

**Connection failed errors**: Make sure your norns is accessible at the endpoint URL. Test with `ping norns.local` and verify port 5555 is open.

**Script not found errors**:
- For simple names like `awake`, ensure the script exists at `dust/code/awake/awake.lua` on your norns
- For paths, verify the path is correct relative to norns's dust directory