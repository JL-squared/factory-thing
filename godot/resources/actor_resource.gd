class_name ActorResource
extends Resource

@export var scene: PackedScene
@export var type: ActorType
@export var name: String

enum ActorType {
	Normal,
	Wire,
	Belt,
	Pipe,
}

# Make sure that every parameter has a default value.
# Otherwise, there will be problems with creating and editing
# your resource via the inspector.
func _init(p_scene: PackedScene = null, p_type: ActorType = ActorType.Normal, p_name: String = "default"):
	type = p_type
	scene = p_scene
	name = p_name
