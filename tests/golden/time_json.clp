<< Golden test: @clpp.time and @clpp.json (deterministic parts only).

link @clpp.time as Time
link @clpp.json as Json

post(Time.format(0))
post(Time.format(1700000000))
dictionary<string, number> day = Time.date(951782400)
post(`${day["year"]}-${day["month"]}-${day["day"]} weekday ${day["weekday"]}`)
float start = Time.clock()
Time.sleep(5)
post(Time.clock() - start >= 0.004)
post(Time.now() > 1700000000.0 and Time.millis() > 1700000000000)

dictionary<string, number> settings = Json.parse("{\"volume\": 7, \"speed\": 1.5, \"name\": \"Ada\", \"tags\": [1, 2], \"on\": true, \"none\": null}")
post(settings["volume"] + 1)
post(settings["speed"] * 2)
post(settings["name"] .: " " .: settings["tags"] .: " " .: settings["on"] .: " " .: settings["none"])
post(Json.stringify(settings))
post(Json.stringify({ Score = 10, Items = { "a", "b" }, Nested = { Ok = true } }))
post(Json.pretty({ Name = "Rui", Levels = { 1, 2 } }))
post(Json.stringify(Json.parse("[1, \"two\", [3.5]]")))
try {
    Json.parse("{ broken")
} catch (error) {
    post("bad json rejected")
}
