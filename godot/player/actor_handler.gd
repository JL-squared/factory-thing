extends Node3D

@onready var camera: Camera3D = $"../Camera3D"
@onready var raycaster: RayCast3D = $"../Camera3D/RayCast3D"
@onready var manager: FactoryManager = get_tree().root.get_child(0).find_child("FactoryManager")

@export var actors: Array[ActorResource]

var highlighted_actor: Node3D = null
var selected_actor_resource: ActorResource = null
var hologram_actor_resource: ActorResource = null
var hologram_actor: Node3D = null
var fst_selected_actor: Node3D = null
var snd_selected_actor: Node3D = null
var selected_actor_index = 0

func _process(delta: float) -> void:
	# returns null if raycast misses
	var possible_actor = null
	if (raycaster.get_collider() != null):
		possible_actor = raycaster.get_collider().get_parent()
	
	# invalidate possible actor if it is not an actor
	if (possible_actor != null && not possible_actor.is_in_group("actors")):
		possible_actor = null
	
	if (possible_actor == null):
		# disable highlight of previous actor when ray cast misses
		if (highlighted_actor != null):
			highlighted_actor.propagate_call("set", ["material_overlay", null])
			highlighted_actor = null
	else:
		if (highlighted_actor == null):
			highlighted_actor = possible_actor;
		elif highlighted_actor != possible_actor:
			highlighted_actor.propagate_call("set", ["material_overlay", null])
			highlighted_actor = possible_actor;
		highlighted_actor.propagate_call("set", ["material_overlay", preload("res://materials/highlight.tres")])
	
	# holograms are only visible when we have a target point where we can place actor
	if (raycaster.get_collider() != null):
		if (hologram_actor_resource != selected_actor_resource or hologram_actor == null):
			recreate_hologram_actor()
		
		
	update_hologram_if_any()
		
func update_hologram_if_any() -> void:
	if (hologram_actor != null):
		hologram_actor.hide()
		
		if (selected_actor_resource == null):
			return;
			
		# only update global position when we are not a wire or belt (because those are automatically handled)
		var selected_actor_type = selected_actor_resource.type
		match selected_actor_type:
			ActorResource.ActorType.Wire:					
				if (fst_selected_actor != null):
					hologram_actor.show()
					var node_p1 = (fst_selected_actor.get_node("Connector") as Node3D).global_position
					
					var node_p2 = -camera.global_basis.z + camera.global_position
					if (raycaster.get_collider() != null):
						var parent = raycaster.get_collider().get_parent()		
						if (parent is PoleNode and parent != fst_selected_actor):
							node_p2 = (parent.get_node("Connector") as Node3D).global_position
					
					var d = node_p1.distance_to(node_p2)
					var pos = (node_p1 + node_p2) * 0.5
					hologram_actor.look_at_from_position(pos, node_p1)
					var mesh = hologram_actor.get_node("MeshInstance3D") as Node3D 
					mesh.scale.y = d * 0.5
			ActorResource.ActorType.Belt:					
				if (fst_selected_actor != null):
					hologram_actor.show()
					var node_p1 = (fst_selected_actor as Node3D).global_position
					
					var node_p2 = -camera.global_basis.z + camera.global_position
					if (raycaster.get_collider() != null):
						var parent = raycaster.get_collider().get_parent()		
						if (parent is HatchNode and parent != fst_selected_actor):
							node_p2 = (parent as Node3D).global_position
					
					var d = node_p1.distance_to(node_p2)
					var pos = (node_p1 + node_p2) * 0.5
					hologram_actor.look_at_from_position(pos, node_p1)
					var mesh = hologram_actor.get_node("Belt Visual") as Node3D 
					mesh.scale.z = d * 0.5
			_:
				if (raycaster.get_collider() != null):
					hologram_actor.show()
					hologram_actor.global_position = get_actor_position_from_raycast()
				pass

func get_actor_position_from_raycast() -> Vector3:
	return round(raycaster.get_collision_point() + Vector3(0, 0.5, 0))

func get_looking_at_actor() -> Node3D:
	if (raycaster.get_collider() != null):
		var parent = raycaster.get_collider().get_parent()
		if (parent.is_in_group("actors")):
			return parent
	return null
	
func get_fst_snd_actor() -> Node3D:
	var actor = get_looking_at_actor()
	
	if (actor is HatchNode or actor is PoleNode):
		return actor
	else:
		return null

func _input(event):
	var actor_index_offset = 0
	
	if event.is_action_pressed("place_actor"):
		if (selected_actor_resource != null):
			var selected_actor_type = selected_actor_resource.type
			match selected_actor_type:
				ActorResource.ActorType.Wire:					
					select_actor()
				ActorResource.ActorType.Belt:					
					select_actor()
				_:
					place_actor()
		
	elif event.is_action_pressed("remove_actor"):
		remove_actor()
	elif event.is_action_pressed("scroll_actor_left"):
		fst_selected_actor = null
		snd_selected_actor = null
		actor_index_offset = -1
	elif event.is_action_pressed("scroll_actor_right"):
		fst_selected_actor = null
		snd_selected_actor = null
		actor_index_offset = 1
	elif event.is_action_pressed("toggle_actor_building"):
		if (selected_actor_resource == null):
			selected_actor_resource = actors[selected_actor_index]
		else:
			selected_actor_resource = null
	if (actor_index_offset != 0):
		selected_actor_index = posmod(selected_actor_index+actor_index_offset, len(actors))
		if (selected_actor_resource != null):
			selected_actor_resource = actors[selected_actor_index]
			print("new actor resource: ", selected_actor_resource.scene.resource_path)

func select_actor() -> void:
	if (fst_selected_actor == null):
		fst_selected_actor = get_fst_snd_actor()
		
		if (fst_selected_actor is HatchNode):
			if ((fst_selected_actor as HatchNode).input_hatch):
				# first hatch is input hatch, which is invalid 
				# we only do belt connections from OUTPUT hatches TOWARDS input hatches
				fst_selected_actor = null
				
		if (fst_selected_actor != null):
			print("selected first actor")
	else:
		snd_selected_actor = get_fst_snd_actor()
		if (snd_selected_actor == fst_selected_actor):
			print("cannot select same actor twice. resetting")
			snd_selected_actor = null
			fst_selected_actor = null
		elif (snd_selected_actor != null):
			print("selected second actor")
			place_actor()	

func recreate_hologram_actor() -> void:
	if (selected_actor_resource == null):
		return
	
	if (hologram_actor != null):
		hologram_actor.queue_free()
		hologram_actor = null
	
	hologram_actor_resource = selected_actor_resource
	match selected_actor_resource.type:
		ActorResource.ActorType.Wire:
			if ((fst_selected_actor as PoleNode) != null):
				hologram_actor = selected_actor_resource.scene.instantiate()
			else:
				return
		ActorResource.ActorType.Belt:
			if ((fst_selected_actor as HatchNode) != null):
				hologram_actor = selected_actor_resource.scene.instantiate()
			else:
				return
		_:
			hologram_actor = selected_actor_resource.scene.instantiate()

	print("recreate hologram actor")
	hologram_actor.find_child("StaticBody3D").process_mode = Node.PROCESS_MODE_DISABLED
	hologram_actor.propagate_call("set", ["process_mode", Node.PROCESS_MODE_DISABLED])
	hologram_actor.propagate_call("set", ["collision_mask", 0])
	hologram_actor.propagate_call("set", ["material_override", preload("res://materials/hologram.tres")])
	
	if ("hologram" in hologram_actor):
		hologram_actor.hologram = true
	
	get_tree().root.get_child(0).add_child(hologram_actor)

		
func place_actor() -> void:
	if (selected_actor_resource == null):
		return
	
	match selected_actor_resource.type:
		ActorResource.ActorType.Wire:
			if ((fst_selected_actor as PoleNode) == null || (snd_selected_actor as PoleNode) == null):
				print("need to select two poles to wire up")
				return
			
			if (manager.are_poles_connected(fst_selected_actor as PoleNode, snd_selected_actor as PoleNode)):
				print("cannot place wire")
				return
				
		ActorResource.ActorType.Belt:
			if ((fst_selected_actor as HatchNode) == null || (snd_selected_actor as HatchNode) == null):
				print("need to select two hatches to belt up")
				return
				
			if (manager.are_hatches_connected(fst_selected_actor as HatchNode, snd_selected_actor as HatchNode)):
				print("cannot place belt")
				return
				
			if (manager.is_hatch_connected(fst_selected_actor as HatchNode)):
				print("cannot place belt")
				return
				
			if (manager.is_hatch_connected(snd_selected_actor as HatchNode)):
				print("cannot place belt")
				return
		_:
			pass
	
	var node = selected_actor_resource.scene.instantiate() as Node3D
	var position = Vector3.ZERO
		
	match selected_actor_resource.type:
		ActorResource.ActorType.Wire:
			var node_p1 = (fst_selected_actor.get_node("Connector") as Node3D).global_position
			var node_p2 = (snd_selected_actor.get_node("Connector") as Node3D).global_position
			var d = node_p1.distance_to(node_p2)
			var pos = (node_p1 + node_p2) * 0.5
			position = pos
			node.look_at_from_position(pos, node_p1)
			var col_shape = node.get_node("StaticBody3D/CollisionShape3D") as CollisionShape3D
			col_shape.shape = col_shape.shape.duplicate()
			(col_shape.shape as BoxShape3D).size.z = d
			
			var mesh = node.get_node("MeshInstance3D") as Node3D 
			mesh.scale.y = d * 0.5
			
			var wire = node as WireNode
			wire.pole_1_ref = fst_selected_actor as PoleNode
			wire.pole_2_ref = snd_selected_actor as PoleNode
			
			
			fst_selected_actor = null
			snd_selected_actor = null
		ActorResource.ActorType.Belt:
			var node_p1 = (fst_selected_actor as Node3D).global_position
			var node_p2 = (snd_selected_actor as Node3D).global_position
			var d = node_p1.distance_to(node_p2)
			var pos = (node_p1 + node_p2) * 0.5
			position = pos
			node.look_at_from_position(pos, node_p1)
			
			var col_shape = node.get_node("StaticBody3D/CollisionShape3D") as CollisionShape3D
			col_shape.shape = col_shape.shape.duplicate()
			(col_shape.shape as BoxShape3D).size.z = d
			
			var mesh = node.get_node("Belt Visual") as Node3D 
			mesh.scale.z = d * 0.5
			
			var belt = node as BeltNode
			belt.belt_start_hatch_ref = fst_selected_actor as HatchNode
			belt.belt_end_hatch_ref = snd_selected_actor as HatchNode
			
			
			fst_selected_actor = null
			snd_selected_actor = null
		_:
			position = get_actor_position_from_raycast()
	
	get_tree().root.get_child(0).add_child(node)
	node.global_position = position
	
	
func remove_actor() -> void: 
	var collider = raycaster.get_collider()
	
	if (collider == null):
		return
	
	var parent = collider.get_parent();
	if (parent.is_in_group("actors")):
		# check if actor is hatch (cannot destroy)
		if (parent is HatchNode):
			return
		
		# check if actor is pole and is owned by machine (and, in which case, we cannot destroy it)
		var pole = parent as PoleNode
		if (pole != null):
			if (!pole.owned):
				parent.queue_free()
		else:
			parent.queue_free()
