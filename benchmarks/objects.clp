<< Objects: construction, field access and method calls.
def Particle {
    float X
    float Speed = 1.5

    void Step() {
        @X += @Speed
    }
}

array<Particle> particles = {}
for (number i in 0 .. 2000) {
    push(particles, Particle { X = i })
}
for (number frame in 0 .. 100) {
    for (Particle p in particles) {
        p.Step()
    }
}
post(particles[10].X)
