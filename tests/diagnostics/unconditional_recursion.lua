-- Error
local function foo()
  foo()
  print("test")
end
foo()

-- Error
local function foo(i)
  foo(i - 1)
  if i == 0 then
    return
  end
  print("test")
end
foo(42)

-- Error
local function foo(i)
  if i == 0 then
    error()
  end
  foo(i - 1)
  print("test")
end
foo(42)

 -- Error
local function foo(i)
  if foo(i) then
    return true
  end
end
foo(42)

-- OK
local function foo(i)
  if i == 0 then
    return
  end
  foo(i - 1)
  print("test")
end
foo(42)

-- OK
local function foo(i)
  print("test")
  coroutine.yield()
  foo(i - 1)
end
foo(42)
