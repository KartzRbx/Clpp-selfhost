<< Golden test: def inheritance, overriding, base defaults, polymorphism.
!as inheritance

link "./lib/shapes.clp" as Shapes

def Entity {
    string Name
    number Health = 100
    array<string> Log

    void Hit(number damage) {
        @Health -= damage
        push(@Log, `hit ${damage}`)
    }

    string Describe() {
        return `${@Name} (${@Health} hp)`
    }
}

def Mage:Entity {
    number Mana

    void Hit(number damage) {
        << A mage shields with mana first.
        if (@Mana >= damage) {
            @Mana -= damage
            push(@Log, `shield ${damage}`)
        } else {
            @Health -= damage
            push(@Log, `hit ${damage}`)
        }
    }


}




def Archmage:Mage {
    number Health = 150

    string Describe() {
        return `Archmage ${@Name} (${@Health} hp, ${@Mana} mana)`
    }
}

def Badge:Shapes.Shape {
    string Owner = "nobody"
}

array<Entity> party = { Entity { Name = "Rui" }, Mage { Name = "Ada" }, Archmage { Name = "Zed", Mana = 5 } }
for (Entity member in party) {
    member.Hit(20)
    member.Hit(40)
    post(member.Describe() .: " " .: member.Log)
}
Entity someone = Mage { Name = "Lia" }
post((Entity someone) == someone)
post((Mage someone).Mana)

Badge badge = Badge { Owner = "Ada" }
post(badge.Describe() .: " owned by " .: badge.Owner)
