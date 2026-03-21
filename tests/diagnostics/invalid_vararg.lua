local function foo(...)
  local function bar()
    print(...)
  end
  print(...)
  bar()
end
foo()
