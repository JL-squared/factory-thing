extends Control

@onready var player: CharacterBody3D = $"../Player"
@onready var actor_handler: = $"../Player/Actor Handler"
@onready var top_label = $"PanelContainer/VBoxContainer/Label"
@onready var recipe_label = $"PanelContainer/VBoxContainer/Label2"
@onready var debug_info_label = $"Debug Label"
@onready var progress_bar = $"PanelContainer/VBoxContainer/ProgressBar"
@onready var selected_actor_label = $"Selected Actor Color Rect/Label2"

func _process(delta: float) -> void:	
	var raycaster: RayCast3D = player.raycaster
	top_label.hide()
	progress_bar.hide()
	recipe_label.hide()
	debug_info_label.hide()
	if (raycaster.get_collider() != null):
		var obj: Node = raycaster.get_collider().get_parent();
		if (obj.has_method("get_ui_info")):
			top_label.show()
			top_label.text = obj.get_ui_info()
		if (obj.has_method("get_ui_progress_bar_percentage")):
			progress_bar.show()
			progress_bar.value = obj.get_ui_progress_bar_percentage()
		if (obj.has_method("get_ui_recipe_info")):
			recipe_label.show()
			recipe_label.text = obj.get_ui_recipe_info()
		if (obj.has_method("get_debug_info")):
			debug_info_label.show()
			debug_info_label.text = obj.get_debug_info()
			
	if (actor_handler.selected_actor_resource != null):
		selected_actor_label.text = actor_handler.selected_actor_resource.name
	else:
		selected_actor_label.text = "Disabled"
