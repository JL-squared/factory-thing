extends Area3D

var attached_rb: RigidBody3D = null
var duration = 0.1

# https://www.reddit.com/r/godot/comments/13uao7l/current_way_in_godot_4_to_find_to_find_player/
@onready var player = get_tree().get_nodes_in_group("player")[0]  
@onready var player_hover_handler = player.find_child("Hover Handler")

signal clicky_thing_attached
signal clicky_thing_detached

func _on_body_entered(body: Node3D) -> void:
	if (attached_rb != null):
		return
	
	if (body is RigidBody3D):
		(body as RigidBody3D).freeze = true
		var tween = get_tree().create_tween()
		tween.tween_property(body, "global_position", global_position, duration).set_trans(Tween.TRANS_SINE)
		tween.tween_property(body, "global_basis", Basis.IDENTITY, duration).set_trans(Tween.TRANS_SINE)
		attached_rb = (body as RigidBody3D)
		
		if (player_hover_handler.rigidbody_hover_target == attached_rb):
			player_hover_handler.rigidbody_hover_target = null
		
		clicky_thing_attached.emit()


func _on_body_exited(body: Node3D) -> void:
	if (body is RigidBody3D):
		if ((body as RigidBody3D) == attached_rb):
			attached_rb = null
			clicky_thing_detached.emit()
	pass # Replace with function body.


func _exit_tree() -> void:
	if (attached_rb != null):
		attached_rb.freeze = false
		clicky_thing_detached.emit()
