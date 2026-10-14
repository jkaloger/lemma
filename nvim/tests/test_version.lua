local T = MiniTest.new_set()

T['plugin loads on the runtimepath'] = function()
  MiniTest.expect.equality(vim.g.loaded_lemma, true)
end

T['version is a semver string'] = function()
  local version = require('lemma.version')
  MiniTest.expect.equality(type(version), 'string')
  MiniTest.expect.equality(version:match('^%d+%.%d+%.%d+$'), version)
end

return T
