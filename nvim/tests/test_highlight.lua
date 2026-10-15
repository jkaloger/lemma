local highlight = require('lemma.highlight')
local eq = MiniTest.expect.equality

local SAMPLE = {
  '## Claims',
  '',
  '- [AC-1] An empty account has balance zero. [intent]',
  '  @ref tests/ledger.rs#balance_sums_entries',
  '  @per RFC 7231 section 4',
  '  @review Says what, not why.',
  '- ~~[CHG-9]~~ [intent] Build: struck rule.',
  '- [AC-DELTA-017-1] [proposed] agent-added claim.',
  '  see @ref src/lib.rs#a here',
  '',
  '> review: Say more about SPEC-004.',
  '',
  'See SPEC-004, DECISION-003 and docs/deltas/DELTA-013-nvim/index.md; not XSPEC-1 or [DICTUM-1].',
}

local function tmpdir()
  local dir = vim.fn.tempname()
  vim.fn.mkdir(dir .. '/docs', 'p')
  return dir
end

local function open_markdown(dir, lines)
  local path = dir .. '/docs/claims.md'
  vim.fn.writefile(lines, path)
  vim.cmd.edit(path)
  local buf = vim.api.nvim_get_current_buf()
  vim.bo[buf].filetype = 'markdown'
  return buf
end

local function marks(buf)
  local out = {}
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local all = vim.api.nvim_buf_get_extmarks(buf, highlight.ns, 0, -1, { details = true })
  for _, m in ipairs(all) do
    local row, col, details = m[2], m[3], m[4]
    out[#out + 1] = {
      group = details.hl_group,
      text = lines[row + 1]:sub(col + 1, details.end_col),
      row = row,
    }
  end
  table.sort(out, function(a, b)
    if a.row ~= b.row then
      return a.row < b.row
    end
    return a.text < b.text
  end)
  return out
end

local function texts(buf, group)
  local out = {}
  for _, m in ipairs(marks(buf)) do
    if m.group == group then
      out[#out + 1] = m.text
    end
  end
  return out
end

local T = MiniTest.new_set({
  hooks = {
    pre_case = function()
      vim.cmd('silent! %bwipeout!')
    end,
  },
})

T['highlights_lemma_syntax_under_a_lazyspec_root'] = function()
  local dir = tmpdir()
  vim.fn.writefile({}, dir .. '/.lazyspec.toml')
  local buf = open_markdown(dir, SAMPLE)

  eq(highlight.attached(buf), true)
  eq(texts(buf, 'LemmaClaimId'), { '[AC-1]', '[AC-DELTA-017-1]', '[DICTUM-1]' })
  eq(texts(buf, 'LemmaStruck'), { '~~[CHG-9]~~' })
  eq(texts(buf, 'LemmaDirective'), { '@ref', '@per', '@review', '@ref' })
  eq(
    texts(buf, 'LemmaRefTarget'),
    { 'tests/ledger.rs#balance_sums_entries', 'RFC 7231 section 4', 'src/lib.rs#a' }
  )
  eq(texts(buf, 'LemmaMarker'), { '[intent]', '[intent]', '[proposed]' })
  eq(texts(buf, 'LemmaReview'), { 'Says what, not why.', '> review: Say more about SPEC-004.' })
  eq(texts(buf, 'LemmaDocId'), { 'DECISION-003', 'DELTA-013', 'SPEC-004' })
end

T['no_highlights_outside_a_lazyspec_root'] = function()
  local buf = open_markdown(tmpdir(), SAMPLE)

  eq(highlight.attached(buf), false)
  eq(marks(buf), {})
end

T['highlights_with_no_lemma_binary_on_path'] = function()
  local saved_path, saved_system = vim.env.PATH, vim.system
  vim.env.PATH = vim.fn.tempname()
  vim.system = function()
    error('highlighting must not spawn a process')
  end
  MiniTest.finally(function()
    vim.env.PATH = saved_path
    vim.system = saved_system
  end)
  eq(vim.fn.executable('lemma'), 0)

  local dir = tmpdir()
  vim.fn.writefile({}, dir .. '/.lazyspec.toml')
  local buf = open_markdown(dir, SAMPLE)

  eq(texts(buf, 'LemmaClaimId'), { '[AC-1]', '[AC-DELTA-017-1]', '[DICTUM-1]' })
  eq(texts(buf, 'LemmaDocId'), { 'DECISION-003', 'DELTA-013', 'SPEC-004' })
end

T['every_group_is_a_default_link_a_user_override_wins'] = function()
  for name, link in pairs(highlight.groups) do
    eq(vim.fn.hlexists(link), 1, name .. ' links to a missing group ' .. link)
    local hl = vim.api.nvim_get_hl(0, { name = name })
    eq({ link = hl.link, default = hl.default }, { link = link, default = true }, name)
  end

  vim.api.nvim_set_hl(0, 'LemmaClaimId', { link = 'Error' })
  highlight.define_groups()
  eq(vim.api.nvim_get_hl(0, { name = 'LemmaClaimId' }).link, 'Error')

  vim.api.nvim_set_hl(0, 'LemmaClaimId', {})
  vim.cmd.hi('clear LemmaClaimId')
  highlight.define_groups()
  eq(vim.api.nvim_get_hl(0, { name = 'LemmaClaimId' }).link, highlight.groups.LemmaClaimId)
end

T['highlights_follow_edits_to_the_buffer'] = function()
  local dir = tmpdir()
  vim.fn.writefile({}, dir .. '/.lazyspec.toml')
  local buf = open_markdown(dir, { '- plain bullet', '- another' })
  eq(marks(buf), {})

  vim.api.nvim_buf_set_lines(buf, 0, 1, false, { '- [AC-1] now a claim' })
  eq(texts(buf, 'LemmaClaimId'), { '[AC-1]' })

  vim.api.nvim_buf_set_lines(buf, 1, 2, false, { '  @ref a.rs#t', '- ~~[AC-2]~~ gone' })
  eq(texts(buf, 'LemmaRefTarget'), { 'a.rs#t' })
  eq(texts(buf, 'LemmaStruck'), { '~~[AC-2]~~' })

  vim.api.nvim_buf_set_text(buf, 0, 2, 0, 8, { 'SPEC-004' })
  eq(texts(buf, 'LemmaClaimId'), {})
  eq(texts(buf, 'LemmaDocId'), { 'SPEC-004' })

  vim.api.nvim_buf_set_lines(buf, 0, 1, false, {})
  eq(texts(buf, 'LemmaDocId'), {})
  eq(texts(buf, 'LemmaRefTarget'), { 'a.rs#t' })
  eq(texts(buf, 'LemmaStruck'), { '~~[AC-2]~~' })
end

return T
