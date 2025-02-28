* Implement proper error handling, not just panic!()
* Implement unit testing, so as to test stuff like this "test" + "abc" == "testabc"
* !synchronize function is broken, printing after an error doesn't work anymore!

* BUG: you can still modify a variable inside a different scope outside of global
lumi> let final a = 1;
lumi> { let b = 2; a = 2; print a; }


* Add continue keyword
* Add switch statements