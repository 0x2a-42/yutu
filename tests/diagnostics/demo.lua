goto foo
local a, b = 1, 2, 3
::foo::
if not (a > b) then
  print(b)
end
