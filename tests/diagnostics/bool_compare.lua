local a = true
local b = nil

if a == true then
  -- warn
end

if a ~= true then
  -- warn
end

if a == false then
  -- warn
end

if a ~= false then
  -- warn
end

if true == a then
  -- warn
end

if true ~= a then
  -- warn
end

if false == a then
  -- warn
end

if false ~= a then
  -- warn
end

if b == true then
  -- don't warn
end

if b ~= true then
  -- don't warn
end

if b == false then
  -- don't warn
end

if b ~= false then
  -- don't warn
end

if true == b then
  -- don't warn
end

if true ~= b then
  -- don't warn
end

if false == b then
  -- don't warn
end

if false ~= b then
  -- don't warn
end
