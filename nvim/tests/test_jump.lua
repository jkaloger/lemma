local eq = MiniTest.expect.equality

local JUMP = vim.api.nvim_replace_termcodes('<Plug>(lemma-jump)', true, false, true)

local function fixture()
  local dir = vim.fn.tempname()
  vim.fn.mkdir(dir, 'p')
  dir = vim.uv.fs_realpath(dir)
  vim.fn.mkdir(dir .. '/docs/specs/SPEC-004-pack', 'p')
  vim.fn.mkdir(dir .. '/tests', 'p')
  vim.fn.writefile({}, dir .. '/.lazyspec.toml')
  vim.fn.writefile({ '# Pack', '', 'Intent.' }, dir .. '/docs/specs/SPEC-004-pack/index.md')
  vim.fn.writefile(
    { 'use ledger::*;', '', '#[test]', 'fn balance_sums_entries() {}', '' },
    dir .. '/tests/ledger.rs'
  )
  vim.fn.writefile({
    '- [AC-1] An empty account has balance zero.',
    '  @ref tests/ledger.rs#balance_sums_entries',
    '  @ref src/lib.rs#a',
    'See SPEC-004 and SPEC-009 and foo.',
    'baz foo',
  }, dir .. '/docs/claims.md')
  vim.cmd.edit(dir .. '/docs/claims.md')
  vim.bo.filetype = 'markdown'
  return dir, vim.api.nvim_get_current_buf()
end

local function entry(kind, line, col, text, status, target)
  return {
    kind = kind,
    line = line,
    col = col,
    end_col = col + #text,
    text = text,
    status = status,
    target = target or vim.NIL,
  }
end

local LISTING = {
  entries = {
    entry(
      'ref',
      2,
      2,
      '@ref tests/ledger.rs#balance_sums_entries',
      'ok',
      { path = 'tests/ledger.rs', line = 4 }
    ),
    entry(
      'ref',
      3,
      2,
      '@ref src/lib.rs#a',
      "AC-1: ref src/lib.rs#a is not in the governing spec's `tests` globs; point it at a test or widen `tests`"
    ),
    entry('doc', 4, 4, 'SPEC-004', 'ok', { path = 'docs/specs/SPEC-004-pack/index.md', line = 1 }),
    entry('doc', 4, 17, 'SPEC-009', 'unknown'),
  },
}

local calls, notices
local saved = {}

local function stub_lemma(listing)
  vim.system = function(cmd, opts, on_exit)
    calls[#calls + 1] = { cmd = cmd, cwd = opts and opts.cwd }
    on_exit({ code = 0, stdout = vim.json.encode(listing), stderr = '' })
  end
end

local function forbid_lemma()
  vim.system = function()
    error('lemma must not be spawned here')
  end
end

local function press_jump()
  vim.api.nvim_feedkeys(JUMP, 'x', false)
end

local function wait_for(predicate)
  eq(vim.wait(2000, predicate, 10), true, 'timed out')
end

local function jump_and_settle(buf)
  local cursor = vim.api.nvim_win_get_cursor(0)
  press_jump()
  wait_for(function()
    return #notices > 0 or vim.api.nvim_get_current_buf() ~= buf
  end)
  return cursor
end

local T = MiniTest.new_set({
  hooks = {
    pre_case = function()
      vim.cmd('silent! %bwipeout!')
      calls, notices = {}, {}
      saved.system, saved.notify, saved.path = vim.system, vim.notify, vim.env.PATH
      vim.notify = function(message, level)
        notices[#notices + 1] = { message = message, level = level }
      end
    end,
    post_case = function()
      vim.system, vim.notify, vim.env.PATH = saved.system, saved.notify, saved.path
    end,
  },
})

T['jumps_to_ref_target'] = function()
  local dir, buf = fixture()
  stub_lemma(LISTING)
  vim.api.nvim_win_set_cursor(0, { 2, 12 })

  jump_and_settle(buf)

  eq(notices, {})
  eq(vim.api.nvim_buf_get_name(0), dir .. '/tests/ledger.rs')
  eq(vim.api.nvim_win_get_cursor(0), { 4, 0 })
  eq(calls, { { cmd = { 'lemma', 'refs', '--json', dir .. '/docs/claims.md' }, cwd = dir } })
end

T['jumps_to_doc_id_target'] = function()
  local dir, buf = fixture()
  stub_lemma(LISTING)
  vim.api.nvim_win_set_cursor(0, { 4, 7 })

  jump_and_settle(buf)

  eq(notices, {})
  eq(vim.api.nvim_buf_get_name(0), dir .. '/docs/specs/SPEC-004-pack/index.md')
  eq(vim.api.nvim_win_get_cursor(0), { 1, 0 })
  eq(#calls, 1)
end

T['unresolved_ref_notifies_the_diagnostic_and_stays'] = function()
  local _, buf = fixture()
  stub_lemma(LISTING)
  vim.api.nvim_win_set_cursor(0, { 3, 10 })

  local cursor = jump_and_settle(buf)

  eq(#notices, 1)
  eq(notices[1].message, 'lemma: ' .. LISTING.entries[2].status)
  eq(notices[1].level, vim.log.levels.ERROR)
  eq(vim.api.nvim_get_current_buf(), buf)
  eq(vim.api.nvim_win_get_cursor(0), cursor)
end

T['unknown_doc_id_notifies_the_diagnostic_and_stays'] = function()
  local _, buf = fixture()
  stub_lemma(LISTING)
  vim.api.nvim_win_set_cursor(0, { 4, 20 })

  local cursor = jump_and_settle(buf)

  eq(#notices, 1)
  eq(notices[1].message, 'lemma: unknown')
  eq(vim.api.nvim_get_current_buf(), buf)
  eq(vim.api.nvim_win_get_cursor(0), cursor)
end

T['plain_word_runs_builtin_gd_without_calling_lemma'] = MiniTest.new_set({
  parametrize = { { 'with lemma on PATH' }, { 'without lemma on PATH' } },
}, {
  function(case)
    local _, buf = fixture()
    forbid_lemma()
    if case == 'without lemma on PATH' then
      vim.env.PATH = vim.fn.tempname()
      eq(vim.fn.executable('lemma'), 0)
    end
    vim.api.nvim_win_set_cursor(0, { 5, 5 })

    press_jump()

    eq(notices, {})
    eq(vim.api.nvim_get_current_buf(), buf)
    eq(vim.api.nvim_win_get_cursor(0), { 4, 30 })
  end,
})

T['modified_buffer_warns_to_save_first_and_stays'] = function()
  local _, buf = fixture()
  forbid_lemma()
  vim.api.nvim_buf_set_lines(buf, 0, 0, false, { '# Claims' })
  eq(vim.bo[buf].modified, true)
  vim.api.nvim_win_set_cursor(0, { 3, 12 })

  local cursor = jump_and_settle(buf)

  eq(#notices, 1)
  eq(notices[1].level, vim.log.levels.WARN)
  eq(notices[1].message:find('save', 1, true) ~= nil, true, notices[1].message)
  eq(vim.api.nvim_get_current_buf(), buf)
  eq(vim.api.nvim_win_get_cursor(0), cursor)
end

local LAZYSPEC_TOML = {
  '[[types]]',
  'name = "spec"',
  'plural = "specs"',
  'dir = "docs/specs"',
  'prefix = "SPEC"',
  'subdirectory = true',
  'authorship = "generated"',
  'staleness = "drift"',
  'lifecycle = { states = ["draft", "active"], edges = [{ from = "draft", to = "active" }] }',
  '',
  '[[types]]',
  'name = "delta"',
  'plural = "deltas"',
  'dir = "docs/deltas"',
  'prefix = "DELTA"',
  'subdirectory = true',
  'authorship = "generated"',
  'lifecycle = { states = ["draft", "in-progress", "complete"], edges = [{ from = "draft", to = "in-progress" }, { from = "in-progress", to = "complete" }] }',
  '',
  '[[relationships]]',
  'name = "changes"',
  'inverse = "changed-by"',
}

local function frontmatter(fields)
  local lines = { '---' }
  vim.list_extend(lines, fields)
  lines[#lines + 1] = '---'
  lines[#lines + 1] = ''
  return lines
end

-- A lazyspec repo of its own: one spec governing `src/` with a `tests` glob, one delta
-- changing it, and a claim whose `@ref` resolves to a test in that glob.
local function lazyspec_repo()
  local dir = vim.fn.tempname()
  vim.fn.mkdir(dir, 'p')
  dir = vim.uv.fs_realpath(dir)
  local spec, delta = dir .. '/docs/specs/SPEC-001-ledger', dir .. '/docs/deltas/DELTA-001-balance'
  vim.fn.mkdir(spec, 'p')
  vim.fn.mkdir(delta, 'p')
  vim.fn.mkdir(dir .. '/tests', 'p')
  vim.fn.writefile(LAZYSPEC_TOML, dir .. '/.lazyspec.toml')
  vim.fn.writefile(
    frontmatter({
      'title: "Ledger"',
      'type: spec',
      'status: active',
      'author: "t"',
      'date: 2026-01-01',
      'tags: []',
      'governs: ["src/**"]',
      'tests: ["tests/**"]',
    }),
    spec .. '/index.md'
  )
  vim.fn.writefile(
    frontmatter({
      'title: "Balance"',
      'type: delta',
      'status: in-progress',
      'author: "t"',
      'date: 2026-01-01',
      'tags: []',
      'related:',
      '- changes: SPEC-001',
    }),
    delta .. '/index.md'
  )
  vim.fn.writefile({
    '- [AC-1] An empty account has balance zero.',
    '  @ref tests/ledger.rs#balance_sums_entries',
  }, delta .. '/claims.md')
  vim.fn.writefile(
    { '#[test]', 'fn other() {}', '', '#[test]', 'fn balance_sums_entries() {}', '' },
    dir .. '/tests/ledger.rs'
  )
  return dir, delta .. '/claims.md'
end

local function prefer_built_lemma()
  local here = vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))
  local built = vim.fs.joinpath(vim.fs.dirname(vim.fs.dirname(here)), 'target', 'debug')
  if vim.fn.executable(vim.fs.joinpath(built, 'lemma')) == 1 then
    vim.env.PATH = built .. ':' .. vim.env.PATH
  end
end

T['jumps_with_the_real_binary_against_a_fixture_repo'] = function()
  prefer_built_lemma()
  local root, claims = lazyspec_repo()
  local probe = vim
    .system({ 'lemma', 'refs', '--json', claims }, { cwd = root, text = true })
    :wait()
  if probe.code ~= 0 then
    MiniTest.skip('no lemma with `refs --json` on PATH: ' .. vim.trim(probe.stderr))
  end

  vim.cmd.edit(claims)
  local buf = vim.api.nvim_get_current_buf()
  vim.api.nvim_win_set_cursor(0, { 2, 10 })

  jump_and_settle(buf)

  eq(notices, {})
  eq(vim.api.nvim_buf_get_name(0), root .. '/tests/ledger.rs')
  eq(vim.api.nvim_win_get_cursor(0), { 5, 0 })
end

return T
