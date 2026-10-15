local highlight = require('lemma.highlight')
local plugin_version = require('lemma.version')

local M = {}

local JUMPABLE = { ref = true, doc = true }

-- nil until the first jump on a ref or doc id probes `lemma --version`; then 'ok' or 'missing'.
local binary

local function notify(message, level)
  vim.notify('lemma: ' .. message, level or vim.log.levels.ERROR)
end

local function warn_if_version_differs(stdout)
  local found = (stdout or ''):match('%d+%.%d+%.%d+')
  if found == plugin_version then
    return
  end
  notify(
    ('binary is %s, plugin is %s; versions should match'):format(found or 'unknown', plugin_version),
    vim.log.levels.WARN
  )
end

-- vim.system raises when the executable is not on PATH.
local function with_binary(on_ready)
  if binary == 'ok' then
    on_ready()
    return
  end
  local spawned = pcall(vim.system, { 'lemma', '--version' }, { text = true }, function(out)
    vim.schedule(function()
      binary = 'ok'
      warn_if_version_differs(out.stdout)
      on_ready()
    end)
  end)
  if spawned then
    return
  end
  binary = 'missing'
  notify('no `lemma` on PATH; ref and doc id jumps are off this session')
end

-- The ref or doc id span under the cursor, or nil. Lexical, so a plain word costs no process.
function M.span_at_cursor()
  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  local line = vim.api.nvim_buf_get_lines(0, row - 1, row, false)[1] or ''
  for _, span in ipairs(highlight.scan(line)) do
    if JUMPABLE[span.kind] and col >= span.start and col < span.stop then
      return span, row, col
    end
  end
  return nil
end

local function entry_at(entries, row, col)
  for _, entry in ipairs(entries) do
    if JUMPABLE[entry.kind] and entry.line == row and col >= entry.col and col < entry.end_col then
      return entry
    end
  end
  return nil
end

local function open_target(root, target)
  vim.cmd.edit(vim.fn.fnameescape(vim.fs.joinpath(root, target.path)))
  local last = vim.api.nvim_buf_line_count(0)
  vim.api.nvim_win_set_cursor(0, { math.min(target.line, last), 0 })
end

local function on_listing(root, row, col, out)
  if out.code ~= 0 then
    notify(vim.trim(out.stderr or ''))
    return
  end
  local ok, listing = pcall(vim.json.decode, out.stdout, { luanil = { object = true } })
  if not ok or type(listing) ~= 'table' then
    notify('could not read `lemma refs --json` output')
    return
  end
  local entry = entry_at(listing.entries or {}, row, col)
  if not entry then
    notify('nothing to jump to under the cursor')
    return
  end
  if entry.status ~= 'ok' or not entry.target then
    notify(entry.status)
    return
  end
  open_target(root, entry.target)
end

function M.jump()
  local span, row, col = M.span_at_cursor()
  if not span then
    vim.cmd('normal! gd')
    return
  end
  if binary == 'missing' then
    return
  end
  local buf = vim.api.nvim_get_current_buf()
  if vim.bo[buf].modified then
    notify('save the buffer first; lemma reads the file on disk', vim.log.levels.WARN)
    return
  end
  local file = vim.api.nvim_buf_get_name(buf)
  local root = file ~= '' and highlight.root(file)
  if not root then
    notify('not a file under a lazyspec root')
    return
  end
  with_binary(function()
    vim.system({ 'lemma', 'refs', '--json', file }, { cwd = root, text = true }, function(out)
      vim.schedule(function()
        on_listing(root, row, col, out)
      end)
    end)
  end)
end

return M
