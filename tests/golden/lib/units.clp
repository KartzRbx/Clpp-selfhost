!as Units

export const number SCALE = 10
export number conversions = 0

export number ToCentimeters(number meters) {
    conversions += 1
    return meters * 100
}

post("units loaded")
