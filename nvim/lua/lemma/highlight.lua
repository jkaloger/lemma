local M = {}

M.ns = vim.api.nvim_create_namespace('lemma')

M.groups = {
  LemmaClaimId = 'Identifier',
  LemmaStruck = 'Comment',
  LemmaDirective = 'Keyword',
  LemmaRefTarget = 'String',
  LemmaMarker = 'Special',
  LemmaReview = 'DiagnosticWarn',
  LemmaDocId = 'Underlined',
}

M.doc_prefixes = { 'SPEC', 'DELTA', 'DECISION', 'CONVENTION', 'DICTUM' }

local CLAIM_ID = '%[%u[%u%d]*%-[%u%d%-]+%]'

function M.define_groups()
  for name, link in pairs(M.groups) do
    vim.api.nvim_set_hl(0, name, { link = link, default = true })
  end
end

function M.root(path)
  local dir = vim.fn.isdirectory(path) == 1 and path or vim.fs.dirname(path)
  local found = vim.fs.find('.lazyspec.toml', { upward = true, path = dir })[1]
  return found and vim.fs.dirname(found) or nil
end

local function overlaps(spans, s, e)
  for _, span in ipairs(spans) do
    if s < span.stop and e > span.start then
      return true
    end
  end
  return false
end

-- Adds a span at 1-indexed inclusive [s, e] unless an earlier span claimed it.
local function add(spans, s, e, group, kind)
  if not s or e < s or overlaps(spans, s - 1, e) then
    return
  end
  spans[#spans + 1] = { start = s - 1, stop = e, group = group, kind = kind }
end

local function review_line(line, spans)
  local s = line:find('^%s*>%s*review:')
  if s then
    add(spans, s, #line, 'LemmaReview', 'review')
  end
end

local function refs(line, spans)
  local at = 1
  while true do
    local s, e = line:find('@ref', at, true)
    if not s then
      return
    end
    add(spans, s, e, 'LemmaDirective', 'directive')
    local as, ae = line:find('^%s+(%S+)', e + 1)
    if as then
      as = line:find('%S', as)
      add(spans, as, ae, 'LemmaRefTarget', 'ref')
    end
    at = e + 1
  end
end

local LINE_DIRECTIVES = {
  ['@per'] = { 'LemmaRefTarget', 'per' },
  ['@review'] = { 'LemmaReview', 'review' },
}

-- `@per` and `@review` are line directives, as lemma reads them; `@ref` may sit anywhere.
local function line_directive(line, spans)
  local _, e, ks, keyword = line:find('^%s*()(@%l+)')
  local arg = keyword and LINE_DIRECTIVES[keyword]
  if not arg then
    return
  end
  add(spans, ks, e, 'LemmaDirective', 'directive')
  local as = line:find('%S', e + 1)
  if as then
    add(spans, as, #line:gsub('%s+$', ''), arg[1], arg[2])
  end
end

local function struck(line, spans)
  local at = 1
  while true do
    local s, e = line:find('~~' .. CLAIM_ID .. '~~', at)
    if not s then
      return
    end
    add(spans, s, e, 'LemmaStruck', 'struck')
    at = e + 1
  end
end

local function claim_ids(line, spans)
  local at = 1
  while true do
    local s, e = line:find(CLAIM_ID, at)
    if not s then
      return
    end
    add(spans, s, e, 'LemmaClaimId', 'claim')
    at = e + 1
  end
end

local function markers(line, spans)
  for _, marker in ipairs({ '%[intent%]', '%[proposed%]' }) do
    local at = 1
    while true do
      local s, e = line:find(marker, at)
      if not s then
        break
      end
      add(spans, s, e, 'LemmaMarker', 'marker')
      at = e + 1
    end
  end
end

local function bracket_depth_at(line)
  local depth, depths = 0, {}
  for i = 1, #line do
    local c = line:sub(i, i)
    if c == '[' then
      depth = depth + 1
    elseif c == ']' and depth > 0 then
      depth = depth - 1
    end
    depths[i] = depth
  end
  return depths
end

local function doc_ids(line, spans)
  local depths = bracket_depth_at(line)
  for _, prefix in ipairs(M.doc_prefixes) do
    local at = 1
    while true do
      local s, e = line:find(prefix .. '%-%d+', at)
      if not s then
        break
      end
      local after_word = s == 1 or not line:sub(s - 1, s - 1):match('%w')
      if after_word and depths[s] == 0 then
        add(spans, s, e, 'LemmaDocId', 'doc')
      end
      at = e + 1
    end
  end
end

-- Every lemma span in `line`: `{ start, stop, group, kind }` with 0-indexed byte
-- columns, `stop` exclusive. Earlier scanners win where spans overlap.
function M.scan(line)
  local spans = {}
  review_line(line, spans)
  line_directive(line, spans)
  refs(line, spans)
  struck(line, spans)
  claim_ids(line, spans)
  markers(line, spans)
  doc_ids(line, spans)
  table.sort(spans, function(a, b)
    return a.start < b.start
  end)
  return spans
end

local function highlight_lines(buf, first, last)
  vim.api.nvim_buf_clear_namespace(buf, M.ns, first, last)
  local lines = vim.api.nvim_buf_get_lines(buf, first, last, false)
  for i, line in ipairs(lines) do
    local row = first + i - 1
    for _, span in ipairs(M.scan(line)) do
      vim.api.nvim_buf_set_extmark(buf, M.ns, row, span.start, {
        end_col = span.stop,
        hl_group = span.group,
        invalidate = true,
        undo_restore = false,
      })
    end
  end
end

local attached = {}

function M.attach(buf)
  buf = buf == 0 and vim.api.nvim_get_current_buf() or buf
  if attached[buf] then
    return
  end
  attached[buf] = true
  M.define_groups()
  highlight_lines(buf, 0, -1)
  vim.api.nvim_buf_attach(buf, false, {
    on_lines = function(_, b, _, first, _, new_last)
      if not attached[b] then
        return true
      end
      highlight_lines(b, first, new_last)
    end,
    on_reload = function(_, b)
      highlight_lines(b, 0, -1)
    end,
    on_detach = function(_, b)
      attached[b] = nil
    end,
  })
end

function M.detach(buf)
  buf = buf == 0 and vim.api.nvim_get_current_buf() or buf
  attached[buf] = nil
  vim.api.nvim_buf_clear_namespace(buf, M.ns, 0, -1)
end

function M.attached(buf)
  return attached[buf == 0 and vim.api.nvim_get_current_buf() or buf] == true
end

function M.buffer_root(buf)
  local name = vim.api.nvim_buf_get_name(buf)
  return M.root(name ~= '' and name or vim.fn.getcwd())
end

function M.setup_buffer(buf)
  if vim.bo[buf].filetype ~= 'markdown' or not M.buffer_root(buf) then
    M.detach(buf)
    return
  end
  M.attach(buf)
end

return M
