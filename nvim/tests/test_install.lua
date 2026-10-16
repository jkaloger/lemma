local eq = MiniTest.expect.equality

local repo_root =
  vim.fs.dirname(vim.fs.dirname(vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))))

local function readme_lazy_spec_source()
  local readme = table.concat(vim.fn.readfile(repo_root .. '/README.md'), '\n')
  local section = readme:match('\n## nvim\n(.-)\n## ')
  return 'return ' .. section:match('```lua\n(.-)\n```')
end

local T = MiniTest.new_set()

T['readme_lazy_spec_installs_the_plugin_from_this_repository'] = function()
  local source = readme_lazy_spec_source()
  eq(assert(loadstring(source))()[1], 'jkaloger/lemma')

  local spec_file = vim.fn.tempname() .. '.lua'
  vim.fn.writefile(vim.split(source, '\n'), spec_file)
  local probe = ([[
    dofile(%q).config({ dir = %q })
    io.stdout:write(tostring(vim.g.loaded_lemma), ' ', vim.fn.maparg('<Plug>(lemma-jump)', 'n') ~= '' and 'mapped' or 'unmapped')
  ]]):format(spec_file, repo_root)
  local out = vim
    .system({ 'nvim', '--headless', '--clean', '-i', 'NONE', '-c', 'lua ' .. probe, '-c', 'qa!' }, {
      text = true,
    })
    :wait()

  eq(out.stdout, 'true mapped')
end

return T
