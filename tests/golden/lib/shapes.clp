!as Shapes

link "./units.clp" as Units

export enum class Kind {
    Square,
    Circle
}

export def Shape {
    string Name = "shape"
    Kind Form = Kind::Square
    number Size = Units.SCALE
    array<string> Tags

    number Area() {
        if (@Form == Kind::Square) {
            return @Size * @Size
        }
        return 3 * @Size * @Size
    }

    string Describe() {
        return @Name .: " (" .: @Form .: ") area " .: @Area()
    }
}

export Shape MakeCircle(string name, number size) {
    return Shape { Name = name, Form = Kind::Circle, Size = size }
}

export number Centimeters(Shape shape) {
    return Units.ToCentimeters(shape.Size)
}

post("shapes loaded")
