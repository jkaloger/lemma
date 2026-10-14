local tests_dir = vim.fs.dirname(debug.getinfo(1, 'S').source:sub(2))
local plugin_dir = vim.fs.dirname(tests_dir)
local mini_dir = vim.fs.joinpath(tests_dir, 'deps', 'mini.nvim')

vim.opt.runtimepath:append(plugin_dir)
vim.opt.runtimepath:append(mini_dir)

vim.cmd('runtime plugin/lemma.lua')

require('mini.test').setup({
  collect = {
    find_files = function()
      return vim.fn.globpath(tests_dir, 'test_*.lua', true, true)
    end,
  },
})
