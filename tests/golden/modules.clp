<< Golden test: link between files (nested links, defs, enums, globals, defaults).

link "./lib/shapes.clp" as Shapes
link "./lib/units.clp" as Units

post("main starts")
Shapes.Shape box = Shapes.Shape {}
post(box.Describe())
Shapes.Shape coin = Shapes.MakeCircle("coin", 2)
post(coin.Describe())
post(coin.Form == Shapes.Kind::Circle)
post(Shapes.Centimeters(coin) .: " cm")
post(Units.ToCentimeters(3) .: " cm")
post("conversions: " .: Units.conversions)
push(box.Tags, "red")
post(box.Tags .: " " .: len(coin.Tags))
post((Shapes.Shape box) == box)
