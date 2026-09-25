extends Node3D

enum HatchType {
	Input,
	Output,
}

@export var type: HatchType

@onready var cyl = $CSGCylinder3D

func _ready() -> void:
	cyl.material=cyl.material.duplicate()
	if (type == HatchType.Input):
		cyl.material.albedo_color = Color.BLACK
	else:
		cyl.material.albedo_color = Color.WHITE
	
	pass # Replace with function body.
