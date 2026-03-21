global pairs
local a<const>
a = 42

global b<const>
b = a

local <const> c
c = b

global <const> d
d = c

for _i = 0, 10 do
  _i = 42
end

for _i, _j in pairs({1, 2, 3}) do
  _i = 42
  _j = 100
end
