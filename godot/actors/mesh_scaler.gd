class_name MeshScaler extends Node

@onready var visual_segment_mesh_instance = $"../Visual";
@onready var collision_shape: CollisionShape3D = $"../StaticBody3D/CollisionShape3D";

func _ready() -> void:
	var parent = get_parent()
	
	var a: Vector3 = Vector3.ZERO
	var b: Vector3 = Vector3.ZERO
	
	if (parent is WireNode):
		var wire = parent as WireNode
		
		if (not wire.hologram):
			a = wire.pole_1_ref.find_child("Connector").global_position
			b = wire.pole_2_ref.find_child("Connector").global_position
	
	if (parent is BeltNode):
		var belt = parent as BeltNode
		if (not belt.hologram):
			a = belt.belt_start_hatch_ref.global_position
			b = belt.belt_end_hatch_ref.global_position
		
	if (parent is PipeNode):
		var pipe = parent as PipeNode
		if (not pipe.hologram):
			a = pipe.pipe_start_hatch_ref.global_position
			b = pipe.pipe_end_hatch_ref.global_position	
			
	if (a != Vector3.ZERO and b != Vector3.ZERO):
		update_data(a, b)

func update_data(node_p1: Vector3, node_p2: Vector3):
	var d = node_p1.distance_to(node_p2)
	var pos = (node_p1 + node_p2) * 0.5
	get_parent().global_position = pos;
	get_parent().global_basis = Basis.looking_at(node_p1-pos);
	
	collision_shape.shape = collision_shape.shape.duplicate()
	var shape = collision_shape.shape
	
	if (shape is BoxShape3D):
		(shape as BoxShape3D).size.z = d
	else:
		(shape as CylinderShape3D).height = d
	
	var mesh = visual_segment_mesh_instance as Node3D 
	mesh.scale.z = d * 0.5
	return
