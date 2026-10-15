if vim.g.loaded_lemma then
  return
end
vim.g.loaded_lemma = true

local group = vim.api.nvim_create_augroup('lemma', { clear = true })

vim.api.nvim_create_autocmd('FileType', {
  group = group,
  pattern = '*',
  callback = function(ev)
    require('lemma.highlight').setup_buffer(ev.buf)
  end,
})

vim.api.nvim_create_autocmd('ColorScheme', {
  group = group,
  callback = function()
    require('lemma.highlight').define_groups()
  end,
})

require('lemma.highlight').define_groups()
